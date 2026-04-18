//! Tauri commands exposed to the frontend for local model lifecycle and chat.

use std::sync::{atomic::Ordering, Arc};
use std::path::PathBuf;

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::model_storage::get_downloaded_models_internal;
use crate::rag::{
    self, RAGConfig,
    chunking::{chunk_code, chunk_text},
    embedding::{batch_generate_embeddings, generate_embedding},
    retrieval::enhance_with_rag,
    vector_storage::VectorStorage,
};

use super::{
    logging::write_log,
    server::{
        kill_server, resolve_candidates, spawn_server, wait_for_health,
        spawn_embedding_server, SERVER_PORT, EMBEDDING_PORT,
    },
    state::LocalChatState,
    types::{AttachedFile, LoadedModelInfo, Message},
};

/// Files under this character count are injected verbatim into the prompt.
/// Files at or above this size are handled via RAG chunking.
const DIRECT_INJECT_CHAR_LIMIT: usize = 40_000; // ~10k tokens, well inside 8k ctx

// -----------------------------------------------------------------------------
// Model lifecycle commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn get_loaded_model(state: State<'_, LocalChatState>) -> Option<LoadedModelInfo> {
    state.loaded.lock().unwrap().clone()
}

#[tauri::command]
pub async fn load_local_model(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    let model_path = get_downloaded_models_internal(&app)
        .into_iter()
        .find(|m| m.model_id == model_id && m.filename == filename)
        .map(|m| m.path)
        .ok_or_else(|| format!("Model '{}/{}' not found.", model_id, filename))?;

    let _ = app.emit("model-loading", serde_json::json!({ "model_id": &model_id, "filename": &filename }));

    let old_child = state.server.lock().unwrap().take();
    if let Some(child) = old_child {
        kill_server(child)?;
    }
    *state.loaded.lock().unwrap() = None;

    let log_path = app
        .path()
        .app_data_dir()
        .map(|d| d.join("llama-server.log"))
        .unwrap_or_else(|_| PathBuf::from("llama-server.log"));

    write_log(&log_path, &format!("load_local_model — model_path={}", model_path));

    let mut candidates = resolve_candidates();
    let cached = state.active_bin.lock().unwrap().clone();

    if let Some(ref cached_name) = cached {
        if let Some(pos) = candidates.iter().position(|c| c.name == cached_name) {
            if pos != 0 {
                let preferred = candidates.remove(pos);
                candidates.insert(0, preferred);
            }
        }
    }

    let mut last_error = String::new();
    for candidate in candidates {
        let _ = app.emit("model-backend-trying", serde_json::json!({ "backend": candidate.backend }));

        let child = match spawn_server(&app, &candidate, &model_path, &log_path).await {
            Ok(c) => c,
            Err(e) => {
                last_error = e.clone();
                write_log(&log_path, &format!("[{}] spawn failed: {}", candidate.backend, e));
                let _ = app.emit("model-backend-failed", serde_json::json!({ "backend": candidate.backend, "reason": &e }));
                continue;
            }
        };

        match wait_for_health(&state.client, SERVER_PORT, candidate.timeout, &log_path, candidate.backend).await {
            Ok(()) => {
                let info = LoadedModelInfo {
                    model_id: model_id.clone(),
                    filename: filename.clone(),
                    backend: candidate.backend.to_string(),
                };
                *state.server.lock().unwrap() = Some(child);
                *state.loaded.lock().unwrap() = Some(info.clone());
                *state.active_bin.lock().unwrap() = Some(candidate.name.to_string());
                let _ = app.emit("model-loaded", &info);
                return Ok(());
            }
            Err(e) => {
                last_error = format!("{} backend: {}", candidate.backend, e);
                let _ = app.emit("model-backend-failed", serde_json::json!({ "backend": candidate.backend, "reason": &last_error }));
                kill_server(child)?;
                if Some(candidate.name) == cached.as_deref() {
                    *state.active_bin.lock().unwrap() = None;
                }
            }
        }
    }

    let e = format!("Failed to start llama-server. Last error: {}", last_error);
    let _ = app.emit("model-error", &e);
    Err(e)
}

#[tauri::command]
pub async fn unload_local_model(state: State<'_, LocalChatState>) -> Result<(), String> {
    if let Some(child) = state.server.lock().unwrap().take() {
        kill_server(child)?;
    }
    *state.loaded.lock().unwrap() = None;
    Ok(())
}

// -----------------------------------------------------------------------------
// Core chat streaming (no RAG)
// -----------------------------------------------------------------------------

async fn start_local_chat_impl(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    messages: Vec<Message>,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.cancel);

    if state.loaded.lock().unwrap().is_none() {
        let msg = "No local model is loaded.";
        let _ = app.emit("local-chat-error", msg);
        return Err(msg.into());
    }

    // FIX 3: llama-server uses the OpenAI messages schema — it only reads
    // `role` and `content`. Serialising the full Message struct sends an
    // `attachments` field the server silently ignores, meaning file content
    // sitting in that field never reaches the model as readable text.
    // Strip it here so only role+content reach the wire.
    let clean_messages: Vec<serde_json::Value> = messages
        .into_iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

    let url = format!("http://127.0.0.1:{}/v1/chat/completions", SERVER_PORT);
    let body = serde_json::to_string(&serde_json::json!({
        "messages": clean_messages,
        "stream": true,
        "max_tokens": 2048,
        "cache_prompt": true,
    })).map_err(|e| e.to_string())?;

    let response = state.client.post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("Could not reach llama-server: {}", e);
            let _ = app.emit("local-chat-error", &msg);
            msg
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        let msg = format!("llama-server error {}: {}", status, body);
        let _ = app.emit("local-chat-error", &msg);
        return Err(msg);
    }

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            let _ = app.emit("local-chat-done", ());
            return Ok(());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            if !line.starts_with("data: ") { continue; }
            let data = line[6..].trim();
            if data == "[DONE]" {
                let _ = app.emit("local-chat-done", ());
                return Ok(());
            }
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                    if !content.is_empty() {
                        let _ = app.emit("local-chat-token", content);
                    }
                }
            }
        }
    }

    let _ = app.emit("local-chat-done", ());
    Ok(())
}

#[tauri::command]
pub async fn start_local_chat(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    messages: Vec<Message>,
) -> Result<(), String> {
    start_local_chat_impl(app, state, messages).await
}

// -----------------------------------------------------------------------------
// Chat with RAG (permanent documents)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn start_local_chat_with_rag(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    messages: Vec<Message>,
    use_rag: bool,
    conversation_id: Option<String>,
) -> Result<(), String> {
    let mut enhanced_messages = messages.clone();
    if use_rag {
        if let Some(last_user_msg) = messages.iter().rev().find(|m| m.role == "user") {
            let config = RAGConfig::default();
            let vector_storage = VectorStorage::new(&app, config.clone())
                .map_err(|e| format!("Vector storage init failed: {}", e))?;

            let log_path = app.path().app_data_dir()
                .map(|d| d.join("rag.log"))
                .unwrap_or_else(|_| PathBuf::from("rag.log"));

            let embedding_port = ensure_embedding_server(&app, &state, &log_path).await?;

            let _ = app.emit("rag-searching", serde_json::json!({ "query": &last_user_msg.content }));

            match enhance_with_rag(
                &app,
                &vector_storage,
                &state.client,
                embedding_port,
                &last_user_msg.content,
                conversation_id.as_deref(),
                &config,
                &log_path,
            ).await {
                Ok(rag_context) => {
                    if !rag_context.relevant_chunks.is_empty() {
                        let _ = app.emit("rag-context-found", serde_json::json!({ "chunk_count": rag_context.relevant_chunks.len() }));
                        let enhanced_prompt = rag::retrieval::build_rag_prompt(&rag_context, &last_user_msg.content);
                        if let Some(last) = enhanced_messages.last_mut() {
                            if last.role == "user" {
                                last.content = enhanced_prompt;
                            }
                        }
                    } else {
                        let _ = app.emit("rag-context-empty", ());
                    }
                }
                Err(e) => {
                    let msg = format!("RAG search failed: {}", e);
                    let _ = app.emit("local-chat-error", &msg);
                }
            }
        }
    }
    start_local_chat_impl(app, state, enhanced_messages).await
}

// -----------------------------------------------------------------------------
// Chat with attachments (on-the-fly RAG)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn start_local_chat_with_attachments(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    messages: Vec<Message>,
    conversation_id: Option<String>,
    attachments: Vec<AttachedFile>,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::SeqCst);

    if state.loaded.lock().unwrap().is_none() {
        let msg = "No local model is loaded.";
        let _ = app.emit("local-chat-error", msg);
        return Err(msg.into());
    }

    let last_user_msg = messages.iter().rev().find(|m| m.role == "user")
        .ok_or("No user message found")?;
    let query = last_user_msg.content.clone();

    let config = RAGConfig::default();
    let log_path = app.path().app_data_dir()
        .map(|d| d.join("rag.log"))
        .unwrap_or_else(|_| PathBuf::from("rag.log"));

    let _ = app.emit("rag-searching", serde_json::json!({
        "query": &query,
        "attachment_count": attachments.len(),
    }));

    // -------------------------------------------------------------------------
    // FIX 1 + 2: Two-path strategy per attached file.
    //
    // SMALL files (< DIRECT_INJECT_CHAR_LIMIT chars):
    //   Inject the full content verbatim. No embedding needed, no similarity
    //   threshold that can silently drop the whole file. The model always
    //   sees 100% of the file regardless of what the user asks.
    //
    // LARGE files (>= DIRECT_INJECT_CHAR_LIMIT chars):
    //   Chunk + embed + rank by cosine similarity, but always include the
    //   top-k results even if scores are below the threshold. A low score
    //   only means the query wording doesn't match the chunk wording; the
    //   content is still what the user explicitly attached and wanted read.
    // -------------------------------------------------------------------------

    let mut direct_context = String::new();     // verbatim content (small files)
    let mut rag_chunks: Vec<String> = Vec::new(); // chunks needing embedding (large files)

    for file in &attachments {
        if file.content.len() < DIRECT_INJECT_CHAR_LIMIT {
            // Small file — always inject in full with filename header
            direct_context.push_str(&format!(
                "### File: `{}` ({})\n```\n{}\n```\n\n",
                file.name, file.r#type, file.content,
            ));
        } else {
            // Large file — chunk and rank semantically
            let chunks = if is_code_file(&file.name) {
                chunk_code(&file.content, config.chunk_size, config.chunk_overlap)
            } else {
                chunk_text(&file.content, config.chunk_size, config.chunk_overlap)
            };
            // Tag each chunk with its filename so the model knows the source
            for chunk in chunks {
                rag_chunks.push(format!("[{}]\n{}", file.name, chunk));
            }
        }
    }

    // Rank large-file chunks but ALWAYS take top-k — no threshold filter
    // (FIX 1: the old `.filter(|(sim, _)| *sim >= threshold)` was the silent
    // killer — it would drop every chunk when queries like "read this file"
    // don't textually resemble the code/doc content).
    let mut rag_context_text = String::new();
    if !rag_chunks.is_empty() {
        let embedding_port = ensure_embedding_server(&app, &state, &log_path).await?;

        let chunk_embeddings =
            batch_generate_embeddings(&state.client, &rag_chunks, embedding_port, &log_path).await?;
        let query_embedding =
            generate_embedding(&state.client, &query, embedding_port).await?;

        let mut scored: Vec<(f32, &str)> = chunk_embeddings
            .iter()
            .zip(rag_chunks.iter())
            .map(|(emb, chunk)| (cosine_similarity(&query_embedding, emb), chunk.as_str()))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(config.top_k); // best k — no score gate

        if !scored.is_empty() {
            rag_context_text.push_str("### Relevant excerpts from large attached files\n\n");
            for (i, (_, chunk)) in scored.iter().enumerate() {
                rag_context_text.push_str(&format!("[Excerpt {}]\n{}\n\n", i + 1, chunk));
            }
        }
    }

    // Search permanent vector storage for this conversation (unchanged logic)
    let mut permanent_context = String::new();
    if let Some(conv_id) = &conversation_id {
        if let Ok(vector_storage) = VectorStorage::new(&app, config.clone()) {
            let embedding_port = ensure_embedding_server(&app, &state, &log_path).await?;
            let permanent_results = vector_storage
                .search(&state.client, embedding_port, &query, Some(conv_id), &log_path)
                .await
                .unwrap_or_default();
            permanent_context = rag::retrieval::format_rag_context(&permanent_results);
        }
    }

    // Merge all context sources
    let combined_context = format!("{}{}{}", direct_context, rag_context_text, permanent_context);

    // Enhance the last user message with the context prefix
    let mut enhanced_messages = messages.clone();
    if !combined_context.trim().is_empty() {
        let enhanced_prompt = format!(
            "{}\n---\nUser question: {}",
            combined_context.trim_end(),
            query,
        );
        if let Some(last) = enhanced_messages.iter_mut().rev().find(|m| m.role == "user") {
            last.content = enhanced_prompt;
        }
        let _ = app.emit("rag-context-found", serde_json::json!({
            "chunk_count": attachments.len(),
        }));
    } else {
        // Shouldn't happen with direct injection, but handle gracefully
        let _ = app.emit("rag-context-empty", ());
    }

    start_local_chat_impl(app, state, enhanced_messages).await
}

#[tauri::command]
pub fn stop_local_chat(state: State<'_, LocalChatState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn is_code_file(filename: &str) -> bool {
    const CODE_EXTENSIONS: &[&str] = &["rs", "py", "js", "ts", "go", "c", "cpp", "h"];
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| CODE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

// -----------------------------------------------------------------------------
// Embedding server management (Send-safe)
// -----------------------------------------------------------------------------

async fn ensure_embedding_server(
    app: &AppHandle,
    state: &LocalChatState,
    log_path: &PathBuf,
) -> Result<u16, String> {
    if *state.embedding_ready.lock().unwrap() {
        return Ok(EMBEDDING_PORT);
    }

    let need_init = {
        let guard = state.embedding_server.lock().unwrap();
        guard.is_none()
    };

    if !need_init {
        while !*state.embedding_ready.lock().unwrap() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        return Ok(EMBEDDING_PORT);
    }

    let embedding_model_path = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("models")
        .join("nomic-embed-text-v1.5.Q4_K_M.gguf")
        .to_string_lossy()
        .to_string();

    if !std::path::Path::new(&embedding_model_path).exists() {
        return Err(format!(
            "Bundled embedding model not found at: {}. \
             Ensure 'models/nomic-embed-text-v1.5.Q4_K_M.gguf' is listed \
             under 'bundle.resources' in tauri.conf.json.",
            embedding_model_path
        ));
    }

    write_log(log_path, &format!("Starting embedding server from bundled model: {}", embedding_model_path));

    let (child, backend) = spawn_embedding_server(app, &embedding_model_path, log_path).await?;

    {
        let mut guard = state.embedding_server.lock().unwrap();
        *guard = Some(child);
    }
    *state.embedding_backend.lock().unwrap() = Some(backend);
    *state.embedding_ready.lock().unwrap() = true;
    write_log(log_path, "Embedding server is ready.");
    Ok(EMBEDDING_PORT)
}