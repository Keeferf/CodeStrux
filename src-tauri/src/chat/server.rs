//! llama-server process management: binary resolution, spawning, health
//! checking, and shutdown.

use std::{path::PathBuf, time::Duration};

use tokio::process::Child;

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

pub const BIN_VULKAN: &str = "llama-server-vulkan.exe";
pub const BIN_CPU: &str    = "llama-server-cpu.exe";

// ── Candidate ─────────────────────────────────────────────────────────────────

/// A candidate binary with its backend label and health-check timeout.
pub struct Candidate {
    pub path:    PathBuf,
    pub backend: &'static str,
    pub timeout: Duration,
}

/// Returns the available binaries from `resource_dir` in preference order
/// (Vulkan first, CPU second), skipping any that are not present on disk.
pub fn resolve_candidates(resource_dir: &std::path::Path) -> Vec<Candidate> {
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

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Strips the `\\?\` extended-length prefix Windows adds to long paths.
///
/// Plain Win32 paths are required for `SetCurrentDirectory` and for
/// `CreateProcess` to resolve the executable's directory (DLL search rule 1).
pub fn strip_verbatim(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return std::path::PathBuf::from(rest.to_string());
    }
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{}", rest));
    }
    path.to_path_buf()
}

// ── Spawn / kill ──────────────────────────────────────────────────────────────

/// Spawns llama-server at `bin` pointing at `model_path`.
///
/// `CREATE_NO_WINDOW` suppresses the console window on Windows.
/// stderr is captured so that if the process crashes before `/health` responds
/// we can include the actual error output in the failure message.
/// `--log-disable` is intentionally omitted so crash output is not suppressed.
pub fn spawn_server(bin: &PathBuf, model_path: &str) -> Result<Child, String> {
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
pub async fn kill_server(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

// ── Health check ──────────────────────────────────────────────────────────────

/// Polls `GET /health` until 200, the process exits, or `timeout` elapses.
///
/// Crash output is written to `log_path` so it can be inspected without
/// surfacing anything in the UI.
pub async fn wait_for_health(
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