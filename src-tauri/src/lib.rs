//! Tauri application entry point.
//!
//! Registers all managed state, plugins, and command handlers, then starts
//! the Tauri event loop.

mod download;
mod hardware;
mod local_chat;
mod model_storage;

use download::DownloadState;
use local_chat::LocalChatState;

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
            local_chat::get_loaded_model,
            local_chat::load_local_model,
            local_chat::unload_local_model,
            local_chat::start_local_chat,
            local_chat::stop_local_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}