//! llama-server process management: binary resolution, spawning, health
//! checking, and shutdown.

use std::time::Duration;

use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

use super::logging::write_log;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const SERVER_PORT: u16 = 28765;

/// Health-check timeout for the Vulkan binary. Shorter than CPU because if
/// Vulkan isn't going to work it usually fails quickly (driver error at init).
pub const VULKAN_HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

/// Health-check timeout for the CPU binary. Large models on a slow machine
/// can take a while to mmap into memory.
pub const CPU_HEALTH_TIMEOUT: Duration = Duration::from_secs(120);

pub const HEALTH_POLL: Duration = Duration::from_millis(250);
pub const CTX_SIZE: u32 = 8192;

// Sidecar names — no .exe, no target triple (Tauri appends both automatically)
pub const BIN_VULKAN: &str = "llama-server-vulkan";
pub const BIN_CPU: &str    = "llama-server-cpu";

// ── Candidate ─────────────────────────────────────────────────────────────────

/// A candidate binary with its backend label and health-check timeout.
/// 
/// `name` is the sidecar name passed to Tauri (no extension, no triple).
pub struct Candidate {
    pub name:    &'static str,   // ← was `path: PathBuf`, Tauri resolves path now
    pub backend: &'static str,
    pub timeout: Duration,
}

/// Returns candidates in preference order (Vulkan first, CPU second).
/// 
/// Unlike before, we don't check disk — Tauri will error at spawn time if
/// a sidecar isn't bundled, which is the right place to surface that.
pub fn resolve_candidates() -> Vec<Candidate> {   // ← no longer needs resource_dir
    vec![
        Candidate { name: BIN_VULKAN, backend: "vulkan", timeout: VULKAN_HEALTH_TIMEOUT },
        Candidate { name: BIN_CPU,    backend: "cpu",    timeout: CPU_HEALTH_TIMEOUT },
    ]
}

// ── Spawn / kill ──────────────────────────────────────────────────────────────

/// Spawns a llama-server sidecar pointing at `model_path`.
///
/// Tauri handles:
/// - Path resolution and DLL discovery
/// - CREATE_NO_WINDOW on Windows
/// - Kill-on-app-exit
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
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", candidate.name, e))?;

    // Stream stderr to log file in the background
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

/// Kills the server. Tauri also does this automatically on app exit.
pub fn kill_server(child: CommandChild) -> Result<(), String> {
    child.kill().map_err(|e| format!("Failed to kill server: {}", e))
}

// ── Health check ──────────────────────────────────────────────────────────────

/// Polls `GET /health` until 200 or `timeout` elapses.
///
/// Note: unlike before, we can no longer fast-fail on process exit by calling
/// `try_wait()` — Tauri's CommandChild doesn't expose that. The timeout
/// handles the crash case instead; stderr is streamed to the log above.
pub async fn wait_for_health(
    client: &reqwest::Client,
    timeout: Duration,
    log_path: &std::path::Path,
    backend: &str,
) -> Result<(), String> {   // ← child param removed, no longer needed
    let url      = format!("http://127.0.0.1:{}/health", SERVER_PORT);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            let msg = format!("llama-server did not become ready within {} s.", timeout.as_secs());
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