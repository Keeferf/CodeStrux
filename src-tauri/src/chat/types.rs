//! Shared data types for local GGUF inference.

use serde::{Deserialize, Serialize};

/// A single message in a chat conversation.
///
/// The shape matches the OpenAI `/v1/chat/completions` messages array so it
/// serialises directly into the llama-server request body.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String,
    pub content: String,
}

/// Metadata for the model currently running in llama-server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadedModelInfo {
    pub model_id: String,
    pub filename: String,
    /// Which backend is running: `"vulkan"` or `"cpu"`.
    pub backend: String,
}