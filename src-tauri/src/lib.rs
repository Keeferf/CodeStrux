//! Tauri application entry point.
//!
//! Registers all managed state, plugins, and command handlers, then starts
//! the Tauri event loop.

mod download;
mod hardware;
mod chat;
mod model_storage;
pub mod chat_storage;
pub mod rag;  // Add RAG module

use tauri::Manager;
use download::DownloadState;
use chat::LocalChatState;

/// Builds and runs the Tauri application.
///
/// # Panics
///
/// Panics if the Tauri runtime fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .manage(DownloadState::default())
        .manage(LocalChatState::default())
        .invoke_handler(tauri::generate_handler![
            // hardware
            hardware::get_hardware_info,
            // downloaded model registry
            model_storage::get_downloaded_models,
            model_storage::delete_downloaded_model,
            // model download
            download::commands::start_download,
            download::commands::cancel_download,
            // local model management + inference
            chat::commands::get_loaded_model,
            chat::commands::load_local_model,
            chat::commands::unload_local_model,
            chat::commands::start_local_chat,
            chat::commands::start_local_chat_with_rag,  // Add RAG-enhanced chat
            chat::commands::stop_local_chat,
            // chat persistence
            chat_storage::create_conversation,
            chat_storage::list_conversations,
            chat_storage::get_conversation_messages,
            chat_storage::append_message,
            chat_storage::rename_conversation,
            chat_storage::delete_conversation,
            // RAG commands
            rag::commands::add_document_to_rag,
            rag::commands::search_documents,
            rag::commands::delete_conversation_rag_documents,
            rag::commands::list_rag_documents,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let app = window.app_handle();
                // Only kill the server when the last window is gone
                if app.webview_windows().is_empty() {
                    if let Some(state) = app.try_state::<LocalChatState>() {
                        if let Some(child) = state.server.lock().unwrap().take() as Option<tauri_plugin_shell::process::CommandChild> {
                            let _ = child.kill();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}