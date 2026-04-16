use crate::rag::Document;

pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = (start + chunk_size).min(text.len());
        let mut break_point = end;

        if end < text.len() {
            if let Some(newline_pos) = text[start..end].rfind('\n') {
                break_point = start + newline_pos + 1;
            } else if let Some(space_pos) = text[start..end].rfind(' ') {
                break_point = start + space_pos + 1;
            } else if let Some(punct_pos) = text[start..end].rfind(|c: char| c == '.' || c == '!' || c == '?') {
                break_point = start + punct_pos + 1;
            }
        }

        chunks.push(text[start..break_point].to_string());

        if break_point >= text.len() {
            break;
        }

        start = break_point.saturating_sub(overlap);
    }

    chunks
}

pub fn chunk_code(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk: Vec<&str> = Vec::new();
    let mut current_size = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_size = line.len() + 1; // +1 for newline

        if current_size + line_size > chunk_size && !current_chunk.is_empty() {
            let split_index = find_split_point(&current_chunk);
            let (to_emit, to_keep) = current_chunk.split_at(split_index);

            chunks.push(to_emit.join("\n"));

            let mut overlap_chunk: Vec<&str> = Vec::new();
            let mut overlap_size = 0;

            for l in to_keep.iter().rev() {
                overlap_size += l.len() + 1;
                overlap_chunk.push(l);
                if overlap_size >= overlap {
                    break;
                }
            }

            overlap_chunk.reverse();
            current_chunk = overlap_chunk;
            current_size = current_chunk.iter().map(|l| l.len() + 1).sum();
        }

        current_chunk.push(line);
        current_size += line_size;

        // Edge case: single line exceeds chunk_size
        if current_size > chunk_size && current_chunk.len() == 1 {
            chunks.push(current_chunk.join("\n"));
            current_chunk.clear();
            current_size = 0;
        }

        // Last line — flush remainder
        if i == lines.len() - 1 && !current_chunk.is_empty() {
            chunks.push(current_chunk.join("\n"));
        }
    }

    chunks
}

pub fn chunk_document_auto(doc: &Document, chunk_size: usize, overlap: usize) -> Vec<String> {
    const CODE_EXTENSIONS: &[&str] = &["rs", "py", "js", "ts", "go", "c", "cpp", "h"];

    if CODE_EXTENSIONS.contains(&doc.metadata.file_type.as_str()) {
        chunk_code(&doc.content, chunk_size, overlap)
    } else {
        chunk_text(&doc.content, chunk_size, overlap)
    }
}

fn find_split_point(chunk: &[&str]) -> usize {
    if chunk.len() <= 1 {
        return 1;
    }
    for i in (1..chunk.len()).rev() {
        let prev_indent = indentation(chunk[i - 1]);
        let curr_indent = indentation(chunk[i]);
        if curr_indent <= prev_indent {
            return i;
        }
    }
    chunk.len().saturating_sub(1)
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}