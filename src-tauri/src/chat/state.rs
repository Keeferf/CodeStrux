//! Runtime state managed by Tauri for the local chat subsystem.

use std::sync::{
    atomic::AtomicBool,
    Arc, Mutex,
};

use tauri_plugin_shell::process::CommandChild;          // ← replaces tokio::process::Child

use super::types::LoadedModelInfo;

pub struct LocalChatState {
    /// The live llama-server child process, if any.
    pub server: Mutex<Option<CommandChild>>,            // ← was Option<Child>

    /// Registry entry for the currently-loaded model.
    pub loaded: Mutex<Option<LoadedModelInfo>>,

    /// The sidecar name that succeeded on the last `load_local_model` call,
    /// cached so we skip the Vulkan probe on subsequent loads.
    pub active_bin: Mutex<Option<String>>,              // ← was Option<PathBuf>

    /// Checked at the top of every SSE chunk loop iteration.
    pub cancel: Arc<AtomicBool>,

    /// Reused for health checks and inference requests.
    pub client: reqwest::Client,
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

impl Drop for LocalChatState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.server.lock() {
            if let Some(child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
}