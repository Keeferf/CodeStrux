//! Vector storage backed by SQLite with cosine similarity

use crate::rag::{Document, DocumentMetadata, RAGConfig, SearchResult};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

/// Vector store backed by SQLite with cosine similarity
/// Wrapped in Arc<Mutex> to make it Send + Sync for async Tauri commands
#[derive(Clone)]
pub struct VectorStorage {
    inner: Arc<Mutex<VectorStorageInner>>,
}

// Explicitly implement Send and Sync for VectorStorage
unsafe impl Send for VectorStorage {}
unsafe impl Sync for VectorStorage {}

struct VectorStorageInner {
    conn: Connection,
    config: RAGConfig,
}

impl VectorStorage {
    pub fn new(app: &AppHandle, config: RAGConfig) -> Result<Self, String> {
        let db_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {}", e))?
            .join("rag");
        
        std::fs::create_dir_all(&db_dir)
            .map_err(|e| format!("Could not create RAG directory: {}", e))?;
        
        let db_path = db_dir.join("vector_storage.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Could not open RAG database: {}", e))?;
        
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             
             CREATE TABLE IF NOT EXISTS documents (
                 id TEXT PRIMARY KEY NOT NULL,
                 content TEXT NOT NULL,
                 filename TEXT NOT NULL,
                 file_type TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 size_bytes INTEGER NOT NULL,
                 conversation_id TEXT,
                 additional_metadata TEXT
             );
             
             CREATE TABLE IF NOT EXISTS embeddings (
                 document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                 chunk_index INTEGER NOT NULL,
                 chunk_text TEXT NOT NULL,
                 embedding BLOB NOT NULL,
                 PRIMARY KEY (document_id, chunk_index)
             );
             
             CREATE INDEX IF NOT EXISTS idx_embeddings_doc ON embeddings(document_id);
             CREATE INDEX IF NOT EXISTS idx_documents_conversation ON documents(conversation_id);
             CREATE INDEX IF NOT EXISTS idx_documents_filetype ON documents(file_type);
             "
        ).map_err(|e| format!("Failed to initialize vector storage schema: {}", e))?;
        
        Ok(Self {
            inner: Arc::new(Mutex::new(VectorStorageInner { conn, config })),
        })
    }
    
    /// Add a document to the vector store with embeddings
    pub async fn add_document(
        &self,
        client: &reqwest::Client,
        server_port: u16,
        doc: Document,
        _log_path: &PathBuf,
    ) -> Result<(), String> {
        let chunks;
        let config;
        {
            let inner = self.inner.lock().unwrap();
            config = inner.config.clone();
            // Chunk the document
            chunks = if doc.metadata.file_type == "rs" || 
                        doc.metadata.file_type == "py" || 
                        doc.metadata.file_type == "js" ||
                        doc.metadata.file_type == "ts" ||
                        doc.metadata.file_type == "go" ||
                        doc.metadata.file_type == "c" ||
                        doc.metadata.file_type == "cpp" ||
                        doc.metadata.file_type == "h" {
                crate::rag::chunking::chunk_code(&doc.content, config.chunk_size, config.chunk_overlap)
            } else {
                crate::rag::chunking::chunk_text(&doc.content, config.chunk_size, config.chunk_overlap)
            };
        }
        
        // Generate embeddings for chunks (this can be done outside the lock)
        let embeddings = crate::rag::embedding::batch_generate_embeddings(
            client,
            &chunks,
            server_port,
            _log_path,
        ).await?;
        
        // Store document metadata - convert u64 to i64 for SQLite
        let size_bytes_i64 = doc.metadata.size_bytes as i64;
        let additional_json = serde_json::to_string(&doc.metadata.additional)
            .unwrap_or_else(|_| "{}".to_string());
        
        let inner = self.inner.lock().unwrap();
        inner.conn.execute(
            "INSERT INTO documents (id, content, filename, file_type, created_at, size_bytes, conversation_id, additional_metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                doc.id,
                doc.content,
                doc.metadata.filename,
                doc.metadata.file_type,
                doc.metadata.created_at,
                size_bytes_i64,
                doc.metadata.conversation_id,
                additional_json,
            ],
        ).map_err(|e| format!("Failed to insert document: {}", e))?;
        
        // Store chunks and embeddings
        for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
            let embedding_blob = inner.serialize_embedding(embedding);
            inner.conn.execute(
                "INSERT INTO embeddings (document_id, chunk_index, chunk_text, embedding)
                 VALUES (?1, ?2, ?3, ?4)",
                params![doc.id, i as i64, chunk, embedding_blob],
            ).map_err(|e| format!("Failed to insert embedding: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Search for relevant chunks using cosine similarity
    pub async fn search(
        &self,
        client: &reqwest::Client,
        server_port: u16,
        query: &str,
        conversation_id: Option<&str>,
        _log_path: &PathBuf,
    ) -> Result<Vec<SearchResult>, String> {
        // Generate embedding for query (outside the lock)
        let query_embedding = crate::rag::embedding::generate_embedding(
            client,
            query,
            server_port,
        ).await?;
        
        let inner = self.inner.lock().unwrap();
        let mut results = Vec::new();
        
        if let Some(conv_id) = conversation_id {
            let mut stmt = inner.conn.prepare(
                "SELECT e.document_id, e.chunk_index, e.chunk_text, e.embedding
                 FROM embeddings e
                 JOIN documents d ON e.document_id = d.id
                 WHERE d.conversation_id = ?1
                 ORDER BY e.document_id, e.chunk_index"
            ).map_err(|e| format!("Failed to prepare search query: {}", e))?;
            
            let rows = stmt.query_map(params![conv_id], |row| {
                let doc_id: String = row.get(0)?;
                let chunk_index: i64 = row.get(1)?;
                let chunk_text: String = row.get(2)?;
                let embedding_blob: Vec<u8> = row.get(3)?;
                Ok((doc_id, chunk_index, chunk_text, embedding_blob))
            }).map_err(|e| format!("Failed to query embeddings: {}", e))?;
            
            for row in rows {
                let (doc_id, _chunk_index, chunk_text, embedding_blob) = row
                    .map_err(|e| format!("Failed to read row: {}", e))?;
                
                let embedding = inner.deserialize_embedding(&embedding_blob);
                let similarity = inner.cosine_similarity(&query_embedding, &embedding);
                
                if similarity >= inner.config.similarity_threshold {
                    let doc_info = inner.get_document_info(&doc_id, conversation_id)?;
                    results.push(SearchResult {
                        document: doc_info,
                        similarity,
                        snippet: inner.generate_snippet(&chunk_text, query),
                    });
                }
            }
        } else {
            let mut stmt = inner.conn.prepare(
                "SELECT e.document_id, e.chunk_index, e.chunk_text, e.embedding
                 FROM embeddings e
                 ORDER BY e.document_id, e.chunk_index"
            ).map_err(|e| format!("Failed to prepare search query: {}", e))?;
            
            let rows = stmt.query_map([], |row| {
                let doc_id: String = row.get(0)?;
                let chunk_index: i64 = row.get(1)?;
                let chunk_text: String = row.get(2)?;
                let embedding_blob: Vec<u8> = row.get(3)?;
                Ok((doc_id, chunk_index, chunk_text, embedding_blob))
            }).map_err(|e| format!("Failed to query embeddings: {}", e))?;
            
            for row in rows {
                let (doc_id, _chunk_index, chunk_text, embedding_blob) = row
                    .map_err(|e| format!("Failed to read row: {}", e))?;
                
                let embedding = inner.deserialize_embedding(&embedding_blob);
                let similarity = inner.cosine_similarity(&query_embedding, &embedding);
                
                if similarity >= inner.config.similarity_threshold {
                    let doc_info = inner.get_document_info(&doc_id, None)?;
                    results.push(SearchResult {
                        document: doc_info,
                        similarity,
                        snippet: inner.generate_snippet(&chunk_text, query),
                    });
                }
            }
        }
        
        // Sort by similarity and take top_k
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(inner.config.top_k);
        
        Ok(results)
    }
    
    /// Delete documents for a conversation
    pub fn delete_conversation_documents(&self, conversation_id: &str) -> Result<(), String> {
        let inner = self.inner.lock().unwrap();
        inner.conn.execute(
            "DELETE FROM documents WHERE conversation_id = ?1",
            params![conversation_id],
        ).map_err(|e| format!("Failed to delete conversation documents: {}", e))?;
        Ok(())
    }
    
    /// Delete a specific document
    pub fn delete_document(&self, document_id: &str) -> Result<(), String> {
        let inner = self.inner.lock().unwrap();
        inner.conn.execute(
            "DELETE FROM documents WHERE id = ?1",
            params![document_id],
        ).map_err(|e| format!("Failed to delete document: {}", e))?;
        Ok(())
    }
    
    /// List all documents (optionally filtered by conversation)
    pub fn list_documents(&self, conversation_id: Option<&str>) -> Result<Vec<Document>, String> {
        let inner = self.inner.lock().unwrap();
        let mut documents = Vec::new();
        
        if let Some(conv_id) = conversation_id {
            let mut stmt = inner.conn.prepare(
                "SELECT id, content, filename, file_type, created_at, size_bytes, additional_metadata
                 FROM documents WHERE conversation_id = ?1"
            ).map_err(|e| format!("Failed to prepare query: {}", e))?;
            
            let rows = stmt.query_map(params![conv_id], |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let filename: String = row.get(2)?;
                let file_type: String = row.get(3)?;
                let created_at: i64 = row.get(4)?;
                let size_bytes_i64: i64 = row.get(5)?;
                let additional_json: String = row.get(6)?;
                let additional: HashMap<String, String> = serde_json::from_str(&additional_json).unwrap_or_default();
                
                Ok((id, content, filename, file_type, created_at, size_bytes_i64, additional))
            }).map_err(|e| format!("Failed to query documents: {}", e))?;
            
            for row in rows {
                let (id, content, filename, file_type, created_at, size_bytes_i64, additional) = row
                    .map_err(|e| format!("Failed to read document row: {}", e))?;
                let size_bytes = size_bytes_i64 as u64;
                
                documents.push(Document {
                    id,
                    content,
                    metadata: DocumentMetadata {
                        filename,
                        file_type,
                        created_at,
                        size_bytes,
                        conversation_id: Some(conv_id.to_string()),
                        additional,
                    },
                    embedding: None,
                });
            }
        } else {
            let mut stmt = inner.conn.prepare(
                "SELECT id, content, filename, file_type, created_at, size_bytes, additional_metadata
                 FROM documents"
            ).map_err(|e| format!("Failed to prepare query: {}", e))?;
            
            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let filename: String = row.get(2)?;
                let file_type: String = row.get(3)?;
                let created_at: i64 = row.get(4)?;
                let size_bytes_i64: i64 = row.get(5)?;
                let additional_json: String = row.get(6)?;
                let additional: HashMap<String, String> = serde_json::from_str(&additional_json).unwrap_or_default();
                
                Ok((id, content, filename, file_type, created_at, size_bytes_i64, additional))
            }).map_err(|e| format!("Failed to query documents: {}", e))?;
            
            for row in rows {
                let (id, content, filename, file_type, created_at, size_bytes_i64, additional) = row
                    .map_err(|e| format!("Failed to read document row: {}", e))?;
                let size_bytes = size_bytes_i64 as u64;
                
                documents.push(Document {
                    id,
                    content,
                    metadata: DocumentMetadata {
                        filename,
                        file_type,
                        created_at,
                        size_bytes,
                        conversation_id: None,
                        additional,
                    },
                    embedding: None,
                });
            }
        }
        
        Ok(documents)
    }
}

// Implement these methods on VectorStorageInner
impl VectorStorageInner {
    /// Serialize embedding vector to bytes for storage
    fn serialize_embedding(&self, embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
    
    /// Deserialize embedding from bytes
    fn deserialize_embedding(&self, bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }
    
    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
    
    /// Generate a snippet around where the query matches
    fn generate_snippet(&self, text: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        
        // Find the best match position
        let text_lower = text.to_lowercase();
        let mut best_pos = None;
        
        for word in words {
            if let Some(pos) = text_lower.find(word) {
                best_pos = Some(pos);
                break;
            }
        }
        
        if let Some(pos) = best_pos {
            let start = pos.saturating_sub(100);
            let end = (pos + 200).min(text.len());
            
            let snippet = &text[start..end];
            if start > 0 {
                format!("...{}...", snippet)
            } else {
                format!("{}...", snippet)
            }
        } else {
            // No match found, just take first 200 chars
            if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text.to_string()
            }
        }
    }
    
    /// Helper to get document info
    fn get_document_info(&self, doc_id: &str, conversation_id: Option<&str>) -> Result<Document, String> {
        let mut stmt = self.conn.prepare(
            "SELECT content, filename, file_type, created_at, size_bytes, additional_metadata
             FROM documents WHERE id = ?1"
        ).map_err(|e| format!("Failed to prepare doc query: {}", e))?;
        
        let row = stmt.query_row(params![doc_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        }).map_err(|e| format!("Failed to get document: {}", e))?;
        
        let (content, filename, file_type, created_at, size_bytes_i64, additional_json) = row;
        let size_bytes = size_bytes_i64 as u64;
        let additional: HashMap<String, String> = 
            serde_json::from_str(&additional_json).unwrap_or_default();
        
        Ok(Document {
            id: doc_id.to_string(),
            content,
            metadata: DocumentMetadata {
                filename,
                file_type,
                created_at,
                size_bytes,
                conversation_id: conversation_id.map(String::from),
                additional,
            },
            embedding: None,
        })
    }
}