//! Runtime state managed by Tauri for the local chat subsystem.

use std::sync::{
    atomic::AtomicBool,
    Arc, Mutex,
};

use tauri_plugin_shell::process::CommandChild;

use super::types::LoadedModelInfo;

pub struct LocalChatState {
    /// The live llama-server child process for chat.
    pub server: Mutex<Option<CommandChild>>,

    /// The live llama-server child process for embeddings.
    pub embedding_server: Mutex<Option<CommandChild>>,

    /// Registry entry for the currently-loaded chat model.
    pub loaded: Mutex<Option<LoadedModelInfo>>,

    /// Whether the embedding server is ready.
    pub embedding_ready: Mutex<bool>,

    /// Backend used for the embedding server ("vulkan" or "cpu").
    pub embedding_backend: Mutex<Option<String>>,

    /// The sidecar name that succeeded on the last `load_local_model` call.
    pub active_bin: Mutex<Option<String>>,

    /// Checked at the top of every SSE chunk loop iteration.
    pub cancel: Arc<AtomicBool>,

    /// Reused for health checks and inference requests.
    pub client: reqwest::Client,
}

impl Default for LocalChatState {
    fn default() -> Self {
        Self {
            server:            Mutex::new(None),
            embedding_server:  Mutex::new(None),
            loaded:            Mutex::new(None),
            embedding_ready:   Mutex::new(false),
            embedding_backend: Mutex::new(None),
            active_bin:        Mutex::new(None),
            cancel:            Arc::new(AtomicBool::new(false)),
            client: reqwest::Client::builder()
                .default_headers({
                    let mut h = reqwest::header::HeaderMap::new();
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
        if let Ok(mut guard) = self.embedding_server.lock() {
            if let Some(child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
}