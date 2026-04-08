//! Text chunking strategies for document processing

use crate::rag::Document;

/// Simple recursive text chunker
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }
    
    let mut chunks = Vec::new();
    let mut start = 0;
    
    while start < text.len() {
        let end = (start + chunk_size).min(text.len());
        
        // Try to find a good break point (newline, space, or punctuation)
        let mut break_point = end;
        if end < text.len() {
            // Look for newline first
            if let Some(newline_pos) = text[start..end].rfind('\n') {
                break_point = start + newline_pos + 1;
            }
            // Then look for space
            else if let Some(space_pos) = text[start..end].rfind(' ') {
                break_point = start + space_pos + 1;
            }
            // Then look for punctuation
            else if let Some(punct_pos) = text[start..end].rfind(|c: char| c == '.' || c == '!' || c == '?') {
                break_point = start + punct_pos + 1;
            }
        }
        
        let chunk = text[start..break_point].to_string();
        chunks.push(chunk);
        
        if break_point >= text.len() {
            break;
        }
        
        // Move start position, accounting for overlap
        start = break_point.saturating_sub(overlap);
    }
    
    chunks
}

/// Chunk documents for embedding
pub fn chunk_document(doc: &Document, chunk_size: usize, overlap: usize) -> Vec<String> {
    chunk_text(&doc.content, chunk_size, overlap)
}

/// Smart chunking with code awareness
pub fn chunk_code(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    // For code, try to split at function/class boundaries
    let mut chunks = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    
    let mut current_chunk: Vec<String> = Vec::new();
    let mut current_len = 0;
    
    for line in lines {
        let line_len = line.len();
        
        // Check if adding this line would exceed chunk size
        if current_len + line_len > chunk_size && !current_chunk.is_empty() {
            // Check if we're at a natural boundary (function/class/impl)
            // Fix: Create a default string outside the condition
            let default_string = String::new();
            let last_line = current_chunk.last().unwrap_or(&default_string);
            
            if last_line.starts_with("fn ") || 
               last_line.starts_with("pub fn ") ||
               last_line.starts_with("struct ") ||
               last_line.starts_with("enum ") ||
               last_line.starts_with("impl ") ||
               last_line.starts_with("trait ") ||
               last_line.starts_with("#[") ||
               last_line.is_empty() {
                
                chunks.push(current_chunk.join("\n"));
                
                // Keep some lines for overlap (rough estimate)
                let keep_lines = (overlap / 50).min(current_chunk.len());
                if keep_lines > 0 {
                    current_chunk = current_chunk
                        .iter()
                        .rev()
                        .take(keep_lines)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>();
                    current_len = current_chunk.iter().map(|l| l.len()).sum();
                } else {
                    current_chunk.clear();
                    current_len = 0;
                }
            }
        }
        
        current_chunk.push(line.to_string());
        current_len += line_len;
    }
    
    if !current_chunk.is_empty() {
        chunks.push(current_chunk.join("\n"));
    }
    
    // If we didn't get any chunks (e.g., all lines too long), fall back to simple chunking
    if chunks.is_empty() {
        chunks = chunk_text(text, chunk_size, overlap);
    }
    
    chunks
}