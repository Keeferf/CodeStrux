use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "codestrux.json";
const TOKEN_KEY: &str = "hf_token";
const MODELS_KEY: &str = "downloaded_models";

// ── Token management ──────────────────────────────────────────────────────────

/// Persist the HF token to the encrypted on-disk store.
/// The token never passes back to the frontend after being saved.
#[tauri::command]
pub fn save_token(app: AppHandle, token: String) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(TOKEN_KEY, serde_json::Value::String(token));
    store.save().map_err(|e| e.to_string())
}

/// Returns true if a non-empty token is stored — frontend only needs to
/// know whether to show the "add token" prompt, never the value itself.
#[tauri::command]
pub fn has_token(app: AppHandle) -> bool {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(TOKEN_KEY))
        .and_then(|v| v.as_str().map(|s| !s.is_empty()))
        .unwrap_or(false)
}

/// Wipe the stored token.
#[tauri::command]
pub fn delete_token(app: AppHandle) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.delete(TOKEN_KEY);
    store.save().map_err(|e| e.to_string())
}

/// Read the token internally (not exposed as a Tauri command).
pub fn read_token(app: &AppHandle) -> Option<String> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(TOKEN_KEY))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
}

// ── Downloaded model registry ─────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredModel {
    pub model_id: String,
    pub filename: String,
    pub path: String,
    pub size: u64,
}

/// Persist a downloaded model entry (upserts by model_id + filename).
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

/// Read the downloaded models list (internal, not a Tauri command).
pub fn get_downloaded_models_internal(app: &AppHandle) -> Vec<StoredModel> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(MODELS_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Return all downloaded models to the frontend.
#[tauri::command]
pub fn get_downloaded_models(app: AppHandle) -> Vec<StoredModel> {
    get_downloaded_models_internal(&app)
}

/// Remove a downloaded model from the registry and delete its file on disk.
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
        // Best-effort file removal
        let _ = std::fs::remove_file(&path);
    }

    Ok(())
}