//! Persistent storage for the downloaded model registry.
//!
//! All reads and writes go through [`tauri_plugin_store`], which encrypts the
//! JSON file on disk.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "codestrux.json";
const MODELS_KEY: &str = "downloaded_models";

// ── Downloaded model registry ─────────────────────────────────────────────────

/// A model file that has been fully downloaded and registered.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredModel {
    pub model_id: String,
    pub filename: String,
    /// Absolute path to the `.gguf` file on disk.
    pub path: String,
    /// File size in bytes.
    pub size: u64,
}

/// Persists a downloaded model entry, upserting by `model_id` + `filename`.
pub fn save_downloaded_model(app: &AppHandle, model: StoredModel) {
    let Ok(store) = app.store(STORE_FILE) else {
        return;
    };
    let mut models = get_downloaded_models_internal(app);
    if let Some(pos) = models
        .iter()
        .position(|m| m.model_id == model.model_id && m.filename == model.filename)
    {
        models[pos] = model;
    } else {
        models.push(model);
    }
    store.set(
        MODELS_KEY,
        serde_json::to_value(&models).unwrap_or_default(),
    );
    let _ = store.save();
}

/// Reads the downloaded models list for internal use.
///
/// Returns an empty list if the store cannot be opened or the key is absent.
pub fn get_downloaded_models_internal(app: &AppHandle) -> Vec<StoredModel> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(MODELS_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Returns all downloaded models to the frontend.
#[tauri::command]
pub fn get_downloaded_models(app: AppHandle) -> Vec<StoredModel> {
    get_downloaded_models_internal(&app)
}

/// Removes a model from the registry and deletes its file from disk.
///
/// Silently succeeds if the model is not found in the registry. File removal
/// is best-effort — a failure there does not fail the command.
///
/// # Errors
///
/// Returns an error if the store cannot be opened or flushed to disk.
#[tauri::command]
pub fn delete_downloaded_model(
    app: AppHandle,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let mut models = get_downloaded_models_internal(&app);

    if let Some(pos) = models
        .iter()
        .position(|m| m.model_id == model_id && m.filename == filename)
    {
        let path = models[pos].path.clone();
        models.remove(pos);
        store.set(
            MODELS_KEY,
            serde_json::to_value(&models).unwrap_or_default(),
        );
        store.save().map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&path);
    }

    Ok(())
}