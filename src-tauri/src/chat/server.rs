//! llama-server process management: binary resolution, spawning, health
//! checking, and shutdown.

use std::time::Duration;

use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

use super::logging::write_log;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const SERVER_PORT: u16 = 28765;
pub const EMBEDDING_PORT: u16 = 28766;   // <-- new

/// Health-check timeout for the Vulkan binary.
pub const VULKAN_HEALTH_TIMEOUT: Duration = Duration::from_secs(60);
/// Health-check timeout for the CPU binary.
pub const CPU_HEALTH_TIMEOUT: Duration = Duration::from_secs(120);
/// Health-check timeout for the embedding server (usually fast).
pub const EMBEDDING_HEALTH_TIMEOUT: Duration = Duration::from_secs(30);

pub const HEALTH_POLL: Duration = Duration::from_millis(250);
pub const CTX_SIZE: u32 = 8192;

// Sidecar names
pub const BIN_VULKAN: &str = "llama-server-vulkan";
pub const BIN_CPU: &str    = "llama-server-cpu";

// ── Candidate ─────────────────────────────────────────────────────────────────

pub struct Candidate {
    pub name:    &'static str,
    pub backend: &'static str,
    pub timeout: Duration,
}

pub fn resolve_candidates() -> Vec<Candidate> {
    vec![
        Candidate { name: BIN_VULKAN, backend: "vulkan", timeout: VULKAN_HEALTH_TIMEOUT },
        Candidate { name: BIN_CPU,    backend: "cpu",    timeout: CPU_HEALTH_TIMEOUT },
    ]
}

// ── Spawn / kill for chat server ─────────────────────────────────────────────

pub async fn spawn_server(
    app_handle: &tauri::AppHandle,
    candidate: &Candidate,
    model_path: &str,
    log_path: &std::path::Path,
) -> Result<CommandChild, String> {
    let (mut rx, child) = app_handle
        .shell()
        .sidecar(candidate.name)
        .map_err(|e| format!("Failed to find sidecar '{}': {}", candidate.name, e))?
        .args([
            "--model",    model_path,
            "--port",     &SERVER_PORT.to_string(),
            "--host",     "127.0.0.1",
            "--ctx-size", &CTX_SIZE.to_string(),
            "--embeddings",
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", candidate.name, e))?;

    let backend   = candidate.backend.to_string();
    let log_path  = log_path.to_path_buf();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stderr(line) = event {
                let line = String::from_utf8_lossy(&line);
                write_log(&log_path, &format!("[{}] {}", backend, line));
            }
        }
    });

    Ok(child)
}

pub fn kill_server(child: CommandChild) -> Result<(), String> {
    child.kill().map_err(|e| format!("Failed to kill server: {}", e))
}

// ── Health check ──────────────────────────────────────────────────────────────

pub async fn wait_for_health(
    client: &reqwest::Client,
    port: u16,
    timeout: Duration,
    log_path: &std::path::Path,
    backend: &str,
) -> Result<(), String> {
    let url      = format!("http://127.0.0.1:{}/health", port);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            let msg = format!("llama-server on port {} did not become ready within {} s.", port, timeout.as_secs());
            write_log(log_path, &format!("[{}] {}", backend, msg));
            return Err(msg);
        }

        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => return Ok(()),
            _ => {}
        }

        tokio::time::sleep(HEALTH_POLL).await;
    }
}

// ── Embedding server ──────────────────────────────────────────────────────────

/// Spawn a dedicated embedding server using the nomic-embed model.
/// Tries Vulkan first, then CPU.
pub async fn spawn_embedding_server(
    app_handle: &tauri::AppHandle,
    embedding_model_path: &str,
    log_path: &std::path::Path,
) -> Result<(CommandChild, String), String> {
    let candidates = resolve_candidates();
    let mut last_error = String::new();

    for candidate in candidates {
        write_log(log_path, &format!("Trying {} for embedding server", candidate.backend));

        let (mut rx, child) = match app_handle
            .shell()
            .sidecar(candidate.name)
            .map_err(|e| format!("Failed to find sidecar '{}': {}", candidate.name, e))?
            .args([
                "--model",    embedding_model_path,
                "--port",     &EMBEDDING_PORT.to_string(),
                "--host",     "127.0.0.1",
                "--ctx-size", "2048",        // smaller context for embeddings
                "--embeddings",
                "--no-mmap",                 // optional: reduce memory
            ])
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                last_error = e.to_string();
                write_log(log_path, &format!("[embedding-{}] spawn failed: {}", candidate.backend, e));
                continue;
            }
        };

        let backend = candidate.backend.to_string();
        let log_path_clone = log_path.to_path_buf();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let CommandEvent::Stderr(line) = event {
                    let line = String::from_utf8_lossy(&line);
                    write_log(&log_path_clone, &format!("[embed-{}] {}", backend, line));
                }
            }
        });

        match wait_for_health(
            &reqwest::Client::new(),
            EMBEDDING_PORT,
            EMBEDDING_HEALTH_TIMEOUT,
            log_path,
            &format!("embed-{}", candidate.backend),
        ).await {
            Ok(()) => {
                write_log(log_path, &format!("Embedding server ready on port {} using {}", EMBEDDING_PORT, candidate.backend));
                return Ok((child, candidate.backend.to_string()));
            }
            Err(e) => {
                last_error = e;
                let _ = child.kill();
                write_log(log_path, &format!("[embed-{}] health check failed: {}", candidate.backend, last_error));
            }
        }
    }

    Err(format!("Failed to start embedding server: {}", last_error))
}