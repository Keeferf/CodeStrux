//! Retrieval system for RAG context

use crate::rag::{RAGConfig, RAGContext, SearchResult, vector_storage::VectorStorage};
use std::path::PathBuf;
use tauri::AppHandle;

/// Format retrieved chunks into a prompt context
pub fn format_rag_context(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    
    let mut context = String::from("Here is relevant information from your documents:\n\n");
    
    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!(
            "[Source {}: {} ({})]\n{}\n\n",
            i + 1,
            result.document.metadata.filename,
            result.document.metadata.file_type,
            result.snippet
        ));
    }
    
    context.push_str("Use this information to answer the user's question. If the information doesn't fully answer the question, use your own knowledge as well.\n\n");
    context
}

/// Enhance a user query with RAG context
pub async fn enhance_with_rag(
    _app: &AppHandle,  // Add underscore
    vector_storage: &VectorStorage,
    client: &reqwest::Client,
    server_port: u16,
    query: &str,
    conversation_id: Option<&str>,
    _config: &RAGConfig,  // Add underscore
    log_path: &PathBuf,
) -> Result<RAGContext, String> {
    // Search for relevant chunks
    let results = vector_storage
        .search(client, server_port, query, conversation_id, log_path)
        .await?;
    
    let context_text = if results.is_empty() {
        String::new()
    } else {
        format_rag_context(&results)
    };
    
    Ok(RAGContext {
        query: query.to_string(),
        relevant_chunks: results,
        context_text,
    })
}

/// Build a prompt that includes RAG context
pub fn build_rag_prompt(context: &RAGContext, original_prompt: &str) -> String {
    if context.context_text.is_empty() {
        return original_prompt.to_string();
    }
    
    format!(
        "{}\n\nUser Question: {}\n\nPlease answer based on the provided context.",
        context.context_text,
        original_prompt
    )
}