//! Tauri commands exposed to the frontend for local model lifecycle and chat.
//!
//! # Lifecycle
//!
//! 1. `load_local_model`   — kills any existing server, tries Vulkan then CPU,
//!                           polls `/health` until ready.
//! 2. `start_local_chat`   — streams `/v1/chat/completions` SSE, emitting
//!                           `local-chat-token` events.
//! 3. `stop_local_chat`    — sets a cancel flag; stream exits cleanly and
//!                           emits `local-chat-done`.
//! 4. `unload_local_model` — kills the server and frees memory.

use std::sync::{atomic::Ordering, Arc};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::model_storage::get_downloaded_models_internal;

use super::{
    logging::write_log,
    server::{kill_server, resolve_candidates, spawn_server, wait_for_health, BIN_CPU, BIN_VULKAN, SERVER_PORT},
    state::LocalChatState,
    types::{LoadedModelInfo, Message},
};

/// Returns the currently-loaded model's metadata, or `None`.
#[tauri::command]
pub fn get_loaded_model(state: State<'_, LocalChatState>) -> Option<LoadedModelInfo> {
    state.loaded.lock().unwrap().clone()
}

/// Loads a downloaded GGUF model by starting (or restarting) llama-server.
///
/// Tries the Vulkan binary first. If its health check times out the process
/// is killed and the CPU binary is tried. On success the winning binary path
/// is cached so future calls on the same machine go straight to the known-good
/// binary.
///
/// Emits:
///
/// | Event                   | Payload                            |
/// |-------------------------|------------------------------------|
/// | `model-loading`         | `{model_id, filename}`             |
/// | `model-backend-trying`  | `{backend}`                        |
/// | `model-backend-failed`  | `{backend, reason}`                |
/// | `model-loaded`          | [`LoadedModelInfo`]                |
/// | `model-error`           | `String`                           |
#[tauri::command]
pub async fn load_local_model(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    // ── 1. Resolve the model path from the registry ───────────────────────────
    let model_path = get_downloaded_models_internal(&app)
        .into_iter()
        .find(|m| m.model_id == model_id && m.filename == filename)
        .map(|m| m.path)
        .ok_or_else(|| {
            format!("Model '{}/{}' not found in the downloaded-model registry.", model_id, filename)
        })?;

    let _ = app.emit("model-loading", serde_json::json!({
        "model_id": &model_id,
        "filename": &filename,
    }));

    // ── 2. Kill any currently-running server ──────────────────────────────────
    // Take the child out of the mutex *before* awaiting so the MutexGuard is
    // dropped immediately. Holding it across `.await` makes the future non-Send.
    let old_child = state.server.lock().unwrap().take();
    if let Some(mut child) = old_child {
        kill_server(&mut child).await;
    }
    *state.loaded.lock().unwrap() = None;

    // ── 3. Resolve candidates ─────────────────────────────────────────────────
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Could not resolve resource directory: {}", e))?;

    let log_path = app
        .path()
        .app_data_dir()
        .map(|d| d.join("llama-server.log"))
        .unwrap_or_else(|_| std::path::PathBuf::from("llama-server.log"));

    write_log(&log_path, &format!(
        "load_local_model called — resource_dir={} model_path={}",
        resource_dir.display(), &model_path
    ));

    let mut candidates = resolve_candidates(&resource_dir);

    if candidates.is_empty() {
        let e = format!(
            "No llama-server binary found in '{}'. \
             Expected '{}' and/or '{}'. \
             See setup instructions in the README.",
            resource_dir.display(), BIN_VULKAN, BIN_CPU
        );
        let _ = app.emit("model-error", &e);
        return Err(e);
    }

    // ── 4. Move the cached binary to the front ────────────────────────────────
    //
    // If we already know which binary works on this machine, try it first to
    // avoid re-probing Vulkan every time the user switches models.
    let cached = state.active_bin.lock().unwrap().clone();
    if let Some(ref cached_path) = cached {
        if let Some(pos) = candidates.iter().position(|c| &c.path == cached_path) {
            if pos != 0 {
                let preferred = candidates.remove(pos);
                candidates.insert(0, preferred);
            }
        }
    }

    // ── 5. Try each candidate in order ────────────────────────────────────────
    let mut last_error = String::new();

    for candidate in candidates {
        let _ = app.emit("model-backend-trying", serde_json::json!({
            "backend": candidate.backend,
        }));

        let mut child = match spawn_server(&candidate.path, &model_path) {
            Ok(c) => c,
            Err(e) => {
                last_error = e.clone();
                write_log(&log_path, &format!("[{}] spawn failed: {}", candidate.backend, &e));
                let _ = app.emit("model-backend-failed", serde_json::json!({
                    "backend": candidate.backend,
                    "reason":  &e,
                }));
                continue;
            }
        };

        match wait_for_health(&state.client, &mut child, candidate.timeout, &log_path, candidate.backend).await {
            Ok(()) => {
                let info = LoadedModelInfo {
                    model_id: model_id.clone(),
                    filename: filename.clone(),
                    backend:  candidate.backend.to_string(),
                };
                *state.server.lock().unwrap()     = Some(child);
                *state.loaded.lock().unwrap()     = Some(info.clone());
                *state.active_bin.lock().unwrap() = Some(candidate.path);
                let _ = app.emit("model-loaded", &info);
                return Ok(());
            }
            Err(e) => {
                last_error = format!("{} backend: {}", candidate.backend, e);
                let _ = app.emit("model-backend-failed", serde_json::json!({
                    "backend": candidate.backend,
                    "reason":  &last_error,
                }));

                kill_server(&mut child).await;

                // If the cached binary just failed (e.g. driver was uninstalled
                // since last run), clear the cache so next time we probe again.
                if cached.as_deref() == Some(candidate.path.as_path()) {
                    *state.active_bin.lock().unwrap() = None;
                }
            }
        }
    }

    // ── All candidates exhausted ──────────────────────────────────────────────
    let e = format!(
        "Failed to start llama-server with any available backend. Last error: {}",
        last_error
    );
    let _ = app.emit("model-error", &e);
    Err(e)
}

/// Kills llama-server and clears loaded-model state.
///
/// `active_bin` is intentionally preserved — we still know which binary works
/// on this machine even after unloading.
#[tauri::command]
pub async fn unload_local_model(state: State<'_, LocalChatState>) -> Result<(), String> {
    let old_child = state.server.lock().unwrap().take();
    if let Some(mut child) = old_child {
        kill_server(&mut child).await;
    }
    *state.loaded.lock().unwrap() = None;
    Ok(())
}

/// Streams a chat completion from the loaded local model.
///
/// Hits `POST /v1/chat/completions` on llama-server's OpenAI-compatible
/// endpoint and forwards SSE tokens to the frontend.
///
/// Emits:
///
/// | Event               | Payload  |
/// |---------------------|----------|
/// | `local-chat-token`  | `String` |
/// | `local-chat-done`   | `()`     |
/// | `local-chat-error`  | `String` |
#[tauri::command]
pub async fn start_local_chat(
    app: AppHandle,
    state: State<'_, LocalChatState>,
    messages: Vec<Message>,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.cancel);

    if state.loaded.lock().unwrap().is_none() {
        let msg = "No local model is loaded. Load one from the Models screen first.";
        let _ = app.emit("local-chat-error", msg);
        return Err(msg.into());
    }

    let url  = format!("http://127.0.0.1:{}/v1/chat/completions", SERVER_PORT);
    let body = serde_json::to_string(&serde_json::json!({
        "messages":     messages,
        "stream":       true,
        "max_tokens":   2048,
        "cache_prompt": true,
    }))
    .map_err(|e| e.to_string())?;

    let response = state
        .client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!(
                "Could not reach llama-server on port {}. \
                 Make sure a model is loaded: {}",
                SERVER_PORT, e
            );
            let _ = app.emit("local-chat-error", &msg);
            msg
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body   = response.text().await.unwrap_or_default();
        let msg    = format!("llama-server error {}: {}", status, body);
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
        let text  = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if !line.starts_with("data: ") {
                continue;
            }
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

/// Signals the running stream to stop after the current chunk.
#[tauri::command]
pub fn stop_local_chat(state: State<'_, LocalChatState>) {
    state.cancel.store(true, Ordering::SeqCst);
}