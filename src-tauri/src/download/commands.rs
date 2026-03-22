use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::model_storage::{save_downloaded_model, StoredModel};

use super::{
    client::{choose_chunks, make_client, probe},
    transfer::{download_parallel, download_stream},
    types::DownloadState,
    validate::{sanitise_filename, sanitise_model_id},
};

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, DownloadState>,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    sanitise_filename(&filename)?;
    let safe_id = sanitise_model_id(&model_id)?;

    let (cancel, _permit) = state.start().map_err(|e| {
        let _ = app.emit("download-error", &e);
        e
    })?;

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model_id, filename
    );

    // No token — only public models are supported.
    let client = Arc::new(make_client(None)?);

    let (resolved_url, total, accepts_ranges) = probe(&client, &url).await.map_err(|e| {
        let _ = app.emit("download-error", &e);
        e
    })?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_dir = data_dir.join("models").join(&safe_id);

    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| e.to_string())?;

    let dest = models_dir.join(&filename);
    let downloaded = Arc::new(AtomicU64::new(0));

    let chunks = if accepts_ranges && total > 0 {
        choose_chunks(total)
    } else {
        1
    };

    let _ = app.emit(
        "download-info",
        serde_json::json!({
            "mode": if chunks > 1 { "parallel" } else { "stream" },
            "total": total,
            "chunks": chunks
        }),
    );

    let result = if chunks > 1 {
        download_parallel(
            app.clone(),
            client,
            resolved_url,
            dest.clone(),
            total,
            downloaded.clone(),
            cancel.clone(),
            model_id.clone(),
            filename.clone(),
            chunks,
        )
        .await
    } else {
        download_stream(
            app.clone(),
            client,
            resolved_url,
            dest.clone(),
            total,
            downloaded.clone(),
            cancel.clone(),
            model_id.clone(),
            filename.clone(),
        )
        .await
    };

    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&dest).await;
        let _ = app.emit("download-error", &e);
        return Err(e);
    }

    if state.is_cancelled() {
        let _ = tokio::fs::remove_file(&dest).await;
        let _ = app.emit("download-cancelled", ());
        return Ok(());
    }

    let final_bytes = downloaded.load(Ordering::SeqCst);
    if total > 0 && final_bytes != total {
        let _ = tokio::fs::remove_file(&dest).await;
        let e = format!("Download incomplete: got {} of {} bytes", final_bytes, total);
        let _ = app.emit("download-error", &e);
        return Err(e);
    }

    let path_str = dest.to_string_lossy().to_string();

    save_downloaded_model(
        &app,
        StoredModel {
            model_id: model_id.clone(),
            filename: filename.clone(),
            path: path_str.clone(),
            size: final_bytes,
        },
    );

    let _ = app.emit(
        "download-done",
        serde_json::json!({
            "model_id": model_id,
            "filename": filename,
            "path": path_str,
        }),
    );

    Ok(())
}

/// Signals the in-progress download to stop at its next cancellation check.
#[tauri::command]
pub fn cancel_download(state: State<'_, DownloadState>) {
    state.request_cancel();
}