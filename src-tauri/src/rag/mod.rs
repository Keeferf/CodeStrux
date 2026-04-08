//! Retrieval-Augmented Generation (RAG) system using nomic-embed
//! 
//! This module provides document indexing, embedding generation, and
//! similarity search to enhance LLM responses with relevant context.

pub mod chunking;
pub mod embedding;
pub mod vector_storage;
pub mod retrieval;
pub mod commands;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for RAG
#[derive(Clone)]
pub struct RAGConfig {
    pub embedding_model: String,
    pub embedding_dim: usize,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    pub similarity_threshold: f32,
}

impl Default for RAGConfig {
    fn default() -> Self {
        Self {
            embedding_model: "nomic-embed".to_string(),
            embedding_dim: 768,
            chunk_size: 512,
            chunk_overlap: 50,
            top_k: 5,
            similarity_threshold: 0.7,
        }
    }
}

/// Document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub filename: String,
    pub file_type: String,
    pub created_at: i64,
    pub size_bytes: u64,
    pub conversation_id: Option<String>,
    pub additional: HashMap<String, String>,
}

/// Search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub similarity: f32,
    pub snippet: String,
}

/// RAG context to be injected into prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGContext {
    pub query: String,
    pub relevant_chunks: Vec<SearchResult>,
    pub context_text: String,
}