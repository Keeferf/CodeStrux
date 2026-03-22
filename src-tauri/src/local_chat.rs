//! Local GGUF inference via a bundled llama-server subprocess.
//!
//! [`resolve_candidates`] returns both binaries in preference order.
//! [`load_local_model`] attempts to start the Vulkan binary first; if its
//! health check times out the process is killed and the CPU binary is tried automatically. The binary that succeeded
//! is cached in [`LocalChatState::active_bin`] so subsequent model loads on
//! the same machine skip the Vulkan probe entirely.
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

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::process::Child;

use crate::model_storage::get_downloaded_models_internal;

// ── Message type ──────────────────────────────────────────────────────────────

/// A single message in a chat conversation.
///
/// Defined here now that `chat.rs` has been removed. The shape matches the
/// OpenAI `/v1/chat/completions` messages array so it serialises directly
/// into the llama-server request body.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String,
    pub content: String,
}

// ── Constants ─────────────────────────────────────────────────────────────────

const SERVER_PORT: u16 = 28765;

/// Health-check timeout for the Vulkan binary. Shorter than CPU because if
/// Vulkan isn't going to work it usually fails quickly (driver error at init).
const VULKAN_HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

/// Health-check timeout for the CPU binary. Large models on a slow machine
/// can take a while to mmap into memory.
const CPU_HEALTH_TIMEOUT: Duration = Duration::from_secs(120);

const HEALTH_POLL: Duration = Duration::from_millis(250);
const CTX_SIZE: u32 = 8192;

const BIN_VULKAN: &str = "llama-server-vulkan.exe";
const BIN_CPU: &str    = "llama-server-cpu.exe";

// ── State ─────────────────────────────────────────────────────────────────────

/// Metadata for the model currently running in llama-server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadedModelInfo {
    pub model_id: String,
    pub filename: String,
    /// Which backend is running: `"vulkan"` or `"cpu"`.
    pub backend: String,
}

pub struct LocalChatState {
    /// The live llama-server child process, if any.
    server: Mutex<Option<Child>>,

    /// Registry entry for the currently-loaded model.
    loaded: Mutex<Option<LoadedModelInfo>>,

    /// The binary that succeeded on the last `load_local_model` call,
    /// cached so we skip the Vulkan probe on subsequent loads.
    active_bin: Mutex<Option<PathBuf>>,

    /// Checked at the top of every SSE chunk loop iteration.
    pub cancel: Arc<AtomicBool>,

    /// Reused for health checks and inference requests.
    client: reqwest::Client,
}

impl Default for LocalChatState {
    fn default() -> Self {
        Self {
            server:     Mutex::new(None),
            loaded:     Mutex::new(None),
            active_bin: Mutex::new(None),
            cancel:     Arc::new(AtomicBool::new(false)),
            client: reqwest::Client::builder()
                .default_headers({
                    let mut h = reqwest::header::HeaderMap::new();
                    // Prevent transparent decompression of the SSE stream.
                    h.insert(
                        reqwest::header::ACCEPT_ENCODING,
                        "identity".parse().unwrap(),
                    );
                    h
                })
                .build()
                .expect("Failed to build local-chat HTTP client"),
        }
    }
}

// ── Binary helpers ────────────────────────────────────────────────────────────

/// A candidate binary with its backend label and health-check timeout.
struct Candidate {
    path:    PathBuf,
    backend: &'static str,
    timeout: Duration,
}

/// Returns the available binaries from `resource_dir` in preference order
/// (Vulkan first, CPU second), skipping any that are not present on disk.
fn resolve_candidates(resource_dir: &std::path::Path) -> Vec<Candidate> {
    [
        (BIN_VULKAN, "vulkan", VULKAN_HEALTH_TIMEOUT),
        (BIN_CPU,    "cpu",    CPU_HEALTH_TIMEOUT),
    ]
    .into_iter()
    .filter_map(|(name, backend, timeout)| {
        let path = resource_dir.join(name);
        path.exists().then_some(Candidate { path, backend, timeout })
    })
    .collect()
}

/// Spawns llama-server at `bin` pointing at `model_path`.
///
/// `CREATE_NO_WINDOW` suppresses the console window on Windows.
///
/// stderr is captured so that if the process crashes before `/health` responds
/// we can include the actual error output in the failure message.
/// `--log-disable` is intentionally omitted so crash output is not suppressed.
/// Strips the `\\?\` extended-length prefix Windows adds to long paths.
///
/// Plain Win32 paths are required for `SetCurrentDirectory` and for
/// `CreateProcess` to resolve the executable's directory (DLL search rule 1).
fn strip_verbatim(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return std::path::PathBuf::from(rest.to_string());
    }
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{}", rest));
    }
    path.to_path_buf()
}

fn spawn_server(bin: &PathBuf, model_path: &str) -> Result<Child, String> {
    let clean_bin = strip_verbatim(bin);
    let clean_dir = clean_bin
        .parent()
        .unwrap_or(clean_bin.as_path())
        .to_path_buf();

    // Prepend the binary's directory to PATH so Windows finds sibling DLLs.
    // Use the string form of clean_dir to guarantee no \\?\ prefix survives
    // into the child process environment.
    let clean_dir_str = clean_dir.to_string_lossy().to_string();
    let path_env = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", clean_dir_str, path_env);

    tokio::process::Command::new(&clean_bin)
        .args([
            "--model",    model_path,
            "--port",     &SERVER_PORT.to_string(),
            "--host",     "127.0.0.1",
            "--ctx-size", &CTX_SIZE.to_string(),
        ])
        .current_dir(&clean_dir)
        .env("PATH", &new_path)
        .kill_on_drop(true)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", clean_bin.display(), e))
}


/// Kills `child` and reaps its exit status.
async fn kill_server(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

/// Appends `message` to `<app_data>/llama-server.log`, creating it if needed.
///
/// Failures are silently ignored — logging must never surface as a user-visible
/// error. The log file is the primary diagnostic tool since errors are
/// intentionally not shown in the UI.
fn write_log(log_path: &std::path::Path, message: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{}] {}", ts, message);
    }
}

/// Polls `GET /health` until 200, the process exits, or `timeout` elapses.
///
/// Crash output is written to `log_path` so it can be inspected without
/// surfacing anything in the UI.
async fn wait_for_health(
    client: &reqwest::Client,
    child: &mut Child,
    timeout: Duration,
    log_path: &std::path::Path,
    backend: &str,
) -> Result<(), String> {
    let url      = format!("http://127.0.0.1:{}/health", SERVER_PORT);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            let msg = format!("llama-server did not become ready within {} s.", timeout.as_secs());
            write_log(log_path, &format!("[{}] {}", backend, msg));
            return Err(msg);
        }

        // Fast-fail: if the process already exited, read stderr and surface it.
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited — collect whatever it wrote to stderr.
                let stderr_text = if let Some(stderr) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut buf = String::new();
                    let mut reader = tokio::io::BufReader::new(stderr);
                    let _ = reader.read_to_string(&mut buf).await;
                    buf.trim().to_string()
                } else {
                    String::new()
                };

                let reason = if stderr_text.is_empty() {
                    format!("exited with status {}", status)
                } else {
                    format!("exited with status {}: {}", status, stderr_text)
                };
                write_log(log_path, &format!("[{}] {}", backend, reason));
                return Err(reason);
            }
            Ok(None) => {} // still running — continue polling
            Err(e)   => return Err(format!("Failed to poll process: {}", e)),
        }

        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => return Ok(()),
            _ => {}
        }

        tokio::time::sleep(HEALTH_POLL).await;
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

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

    // Resolve the log file path — written to app_data so it survives across runs.
    // Open it before the candidate loop so every attempt is captured in one file.
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
            // Only move it if it's not already first.
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

        // Spawn the server process.
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

        // Wait for the server to signal readiness.
        // The child is kept local here so we can pass &mut to wait_for_health.
        // It is moved into state only on success.
        match wait_for_health(&state.client, &mut child, candidate.timeout, &log_path, candidate.backend).await {
            Ok(()) => {
                // ── Binary worked — store child and return ────────────────────
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
                // ── Binary failed — kill it and try the next ──────────────────
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