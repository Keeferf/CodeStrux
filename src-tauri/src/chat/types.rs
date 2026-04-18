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
    /// Optional file attachments for user messages.
    #[serde(default)]
    pub attachments: Option<Vec<AttachedFile>>,
}

/// Metadata for the model currently running in llama-server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoadedModelInfo {
    pub model_id: String,
    pub filename: String,
    /// Which backend is running: `"vulkan"` or `"cpu"`.
    pub backend: String,
}

/// A file attached to a user message.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachedFile {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub size: u64,
    pub content: String,
}