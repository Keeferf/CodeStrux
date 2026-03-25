//! Local GGUF inference via a bundled llama-server subprocess.
//!
//! [`server::resolve_candidates`] returns both binaries in preference order.
//! [`commands::load_local_model`] attempts to start the Vulkan binary first;
//! if its health check times out the process is killed and the CPU binary is
//! tried automatically. The binary that succeeded is cached in
//! [`state::LocalChatState::active_bin`] so subsequent model loads on the same
//! machine skip the Vulkan probe entirely.

pub mod commands;
pub mod logging;
pub mod server;
pub mod state;
pub mod types;

// Re-export LocalChatState so lib.rs can write `chat::LocalChatState`
// instead of `chat::state::LocalChatState`.
pub use state::LocalChatState;