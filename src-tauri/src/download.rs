use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::store::{read_token, save_downloaded_model, StoredModel};

const MAX_PARALLEL_CHUNKS: u64 = 24;
const MAX_RETRIES: usize = 3;

// ── Shared cancel flag ────────────────────────────────────────────────────────

pub struct DownloadState {
    pub cancel: Arc<AtomicBool>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

// ── Event payloads ────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub model_id: String,
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_client(token: Option<&str>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(t) = token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", t).parse().unwrap(),
        );
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .pool_max_idle_per_host(32)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .gzip(true)
        .brotli(true)
        .build()
        .expect("reqwest client")
}

async fn probe(client: &reqwest::Client, url: &str) -> Result<(String, u64, bool), String> {
    let resp = client
        .get(url)
        .header("Range", "bytes=0-0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() && resp.status().as_u16() != 206 {
        return Err(format!("Probe failed: HTTP {}", resp.status()));
    }

    let resolved = resp.url().to_string();

    let accepts_ranges = resp.status().as_u16() == 206;

    let total = resp
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').last())
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| resp.content_length())
        .unwrap_or(0);

    Ok((resolved, total, accepts_ranges))
}

// dynamic chunk count based on file size
fn choose_chunks(total: u64) -> u64 {
    let gb = total as f64 / 1_000_000_000.0;

    if gb < 1.0 {
        8
    } else if gb < 5.0 {
        16
    } else {
        MAX_PARALLEL_CHUNKS
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, DownloadState>,
    model_id: String,
    filename: String,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.cancel);

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model_id, filename
    );

    let token = read_token(&app);
    let client = Arc::new(make_client(token.as_deref()));

    let (resolved_url, total, accepts_ranges) =
        probe(&client, &url).await.map_err(|e| {
            let _ = app.emit("download-error", &e);
            e
        })?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let safe_id = model_id.replace('/', "__");
    let models_dir = data_dir.join("models").join(&safe_id);

    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

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

    if cancel.load(Ordering::SeqCst) {
        let _ = tokio::fs::remove_file(&dest).await;
        let _ = app.emit("download-cancelled", ());
        return Ok(());
    }

    let final_bytes = downloaded.load(Ordering::SeqCst);
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

async fn download_parallel(
    app: AppHandle,
    client: Arc<reqwest::Client>,
    url: String,
    dest: std::path::PathBuf,
    total: u64,
    downloaded: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    model_id: String,
    filename: String,
    chunks: u64,
) -> Result<(), String> {

    {
        let f = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| e.to_string())?;
        f.set_len(total).await.map_err(|e| e.to_string())?;
    }

    let chunk_size = (total + chunks - 1) / chunks;
    let mut tasks = Vec::new();

    for i in 0..chunks {

        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size - 1).min(total - 1);

        if start >= total {
            break;
        }

        let client = Arc::clone(&client);
        let url = url.clone();
        let dest = dest.clone();
        let downloaded = Arc::clone(&downloaded);
        let cancel = Arc::clone(&cancel);
        let app = app.clone();
        let model_id = model_id.clone();
        let filename = filename.clone();

        tasks.push(tokio::spawn(async move {

            let mut attempt = 0;

            loop {

                let resp = client
                    .get(&url)
                    .header("Range", format!("bytes={}-{}", start, end))
                    .send()
                    .await;

                let resp = match resp {
                    Ok(r) => r,
                    Err(e) => {
                        attempt += 1;
                        if attempt >= MAX_RETRIES {
                            return Err(e.to_string());
                        }
                        continue;
                    }
                };

                if !resp.status().is_success() && resp.status().as_u16() != 206 {
                    attempt += 1;

                    if attempt >= MAX_RETRIES {
                        return Err(format!("Chunk {} failed: HTTP {}", i, resp.status()));
                    }

                    continue;
                }

                let mut file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .open(&dest)
                    .await
                    .map_err(|e| e.to_string())?;

                file.seek(std::io::SeekFrom::Start(start))
                    .await
                    .map_err(|e| e.to_string())?;

                let mut stream = resp.bytes_stream();

                while let Some(chunk) = stream.next().await {

                    if cancel.load(Ordering::SeqCst) {
                        return Ok(());
                    }

                    let chunk = chunk.map_err(|e| e.to_string())?;

                    file.write_all(&chunk).await.map_err(|e| e.to_string())?;

                    let so_far = downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed)
                        + chunk.len() as u64;

                    let percent = (so_far as f64 / total as f64) * 100.0;

                    let _ = app.emit(
                        "download-progress",
                        DownloadProgress {
                            model_id: model_id.clone(),
                            filename: filename.clone(),
                            downloaded: so_far,
                            total,
                            percent,
                        },
                    );
                }

                file.flush().await.map_err(|e| e.to_string())?;

                return Ok::<(), String>(());
            }
        }));
    }

    for task in tasks {
        task.await.map_err(|e| e.to_string())??;
    }

    Ok(())
}

async fn download_stream(
    app: AppHandle,
    client: Arc<reqwest::Client>,
    url: String,
    dest: std::path::PathBuf,
    total: u64,
    downloaded: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    model_id: String,
    filename: String,
) -> Result<(), String> {

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {

        if cancel.load(Ordering::SeqCst) {
            return Ok(());
        }

        let chunk = chunk.map_err(|e| e.to_string())?;

        file.write_all(&chunk).await.map_err(|e| e.to_string())?;

        let so_far = downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed)
            + chunk.len() as u64;

        let percent = if total > 0 {
            (so_far as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                model_id: model_id.clone(),
                filename: filename.clone(),
                downloaded: so_far,
                total,
                percent,
            },
        );
    }

    file.flush().await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, DownloadState>) {
    state.cancel.store(true, Ordering::SeqCst);
}