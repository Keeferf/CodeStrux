//! Tauri commands for RAG functionality

use crate::rag::RAGState;
use crate::rag::DocumentMetadata;
use crate::chat::server::SERVER_PORT;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, State, Manager};

use crate::chat::state::LocalChatState;

/// Add a document to the RAG system
#[tauri::command]
pub async fn add_document_to_rag(
    app: AppHandle,
    chat_state: State<'_, LocalChatState>,
    rag_state: State<'_, RAGState>,
    file_path: String,
    conversation_id: Option<String>,
) -> Result<(), String> {
    // Read the file
    let path = PathBuf::from(&file_path);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let filename = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let file_type = path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    // Create document
    let doc_id = format!("doc_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    
    let metadata = DocumentMetadata {
        filename,
        file_type: file_type.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        size_bytes: content.len() as u64,
        conversation_id,
        additional: HashMap::new(),
    };
    
    let document = crate::rag::Document {
        id: doc_id,
        content,
        metadata,
        embedding: None,
    };
    
    // Reuse the shared VectorStorage connection from managed state
    let vector_storage = rag_state.get_or_init(&app)?;
    
    // Get log path
    let log_path = app
        .path()
        .app_data_dir()
        .map(|d| d.join("rag.log"))
        .unwrap_or_else(|_| PathBuf::from("rag.log"));
    
    // Add document to vector storage
    vector_storage.add_document(
        &chat_state.client,
        SERVER_PORT,
        document,
        &log_path,
    ).await?;
    
    Ok(())
}

/// Search documents with RAG
#[tauri::command]
pub async fn search_documents(
    app: AppHandle,
    chat_state: State<'_, LocalChatState>,
    rag_state: State<'_, RAGState>,
    query: String,
    conversation_id: Option<String>,
) -> Result<Vec<crate::rag::SearchResult>, String> {
    let vector_storage = rag_state.get_or_init(&app)?;
    
    let log_path = app
        .path()
        .app_data_dir()
        .map(|d| d.join("rag.log"))
        .unwrap_or_else(|_| PathBuf::from("rag.log"));
    
    let results = vector_storage.search(
        &chat_state.client,
        SERVER_PORT,
        &query,
        conversation_id.as_deref(),
        &log_path,
    ).await?;
    
    Ok(results)
}

/// Delete all documents for a conversation
#[tauri::command]
pub fn delete_conversation_rag_documents(
    app: AppHandle,
    rag_state: State<'_, RAGState>,
    conversation_id: String,
) -> Result<(), String> {
    let vector_storage = rag_state.get_or_init(&app)?;
    vector_storage.delete_conversation_documents(&conversation_id)?;
    Ok(())
}

/// Delete a specific document
#[tauri::command]
pub fn delete_rag_document(
    app: AppHandle,
    rag_state: State<'_, RAGState>,
    document_id: String,
) -> Result<(), String> {
    let vector_storage = rag_state.get_or_init(&app)?;
    vector_storage.delete_document(&document_id)?;
    Ok(())
}

/// List all documents in the RAG system
#[tauri::command]
pub fn list_rag_documents(
    app: AppHandle,
    rag_state: State<'_, RAGState>,
    conversation_id: Option<String>,
) -> Result<Vec<crate::rag::Document>, String> {
    let vector_storage = rag_state.get_or_init(&app)?;
    vector_storage.list_documents(conversation_id.as_deref())
}