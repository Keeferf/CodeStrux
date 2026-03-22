use std::{
    io::{BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use bytes::Bytes;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::client::is_allowed_host;
use super::types::{DownloadProgress, SpeedTracker};

const MAX_CONNECT_RETRIES: u32 = 3;
const MAX_STREAM_RETRIES: u32 = 3;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
// Fires on a live-but-silent connection that would otherwise never trigger
// stream_error or retry logic. Distinct from the 15 s connect timeout.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
// Enough to buffer a few network frames ahead of the disk writer without
// holding excessive data in memory.
const WRITER_CHANNEL_CAPACITY: usize = 32;
// Coalesces many small HTTP frames (typically 16–64 KB) into fewer, larger
// syscalls.
const WRITE_BUF_SIZE: usize = 512 * 1024;

/// Emits `download-progress` events at a fixed interval until the download
/// completes, is cancelled, or the returned handle is aborted.
///
/// [`SpeedTracker`] lives entirely inside this task so no locking is needed.
fn spawn_progress_reporter(
    app: AppHandle,
    downloaded: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    model_id: String,
    filename: String,
    total: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PROGRESS_INTERVAL);
        let mut tracker = SpeedTracker::new();

        loop {
            interval.tick().await;
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            let so_far = downloaded.load(Ordering::Relaxed);
            let percent = if total > 0 {
                (so_far as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            let speed_bps = tracker.record(so_far);
            let eta_secs = match (speed_bps, total) {
                (Some(spd), t) if t > 0 && spd > 0.0 => {
                    let remaining = t.saturating_sub(so_far) as f64;
                    Some(remaining / spd)
                }
                _ => None,
            };

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    model_id: model_id.clone(),
                    filename: filename.clone(),
                    downloaded: so_far,
                    total,
                    percent,
                    speed_bps,
                    eta_secs,
                },
            );

            if total > 0 && so_far >= total {
                break;
            }
        }
    })
}

/// Spawns a dedicated blocking writer thread for one chunk attempt.
///
/// Uses `std::fs`/`std::io` rather than `tokio::fs::File` deliberately:
/// `tokio::fs::File` routes every flush through `spawn_blocking`, which parks
/// the async task waiting for a thread-pool slot. That suspends the TCP
/// receive loop and lets the socket buffer fill, throttling the sender. A
/// single dedicated blocking thread per chunk avoids this overhead entirely.
fn spawn_chunk_writer(
    dest: PathBuf,
    write_from: u64,
    mut rx: mpsc::Receiver<Bytes>,
) -> tokio::task::JoinHandle<Result<(), String>> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .write(true)
            // `create(true)` without `truncate(true)`: creates the file when
            // absent (stream mode); opens without truncating when present
            // (parallel mode, where the file is pre-allocated).
            .create(true)
            .open(&dest)
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::with_capacity(WRITE_BUF_SIZE, file);
        writer
            .seek(SeekFrom::Start(write_from))
            .map_err(|e| e.to_string())?;

        // `blocking_recv` parks the OS thread, not a tokio task, so it is
        // safe to call from `spawn_blocking`.
        while let Some(data) = rx.blocking_recv() {
            writer.write_all(&data).map_err(|e| e.to_string())?;
        }
        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub async fn download_parallel(
    app: AppHandle,
    client: Arc<reqwest::Client>,
    url: String,
    dest: PathBuf,
    total: u64,
    downloaded: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    model_id: String,
    filename: String,
    chunks: u64,
) -> Result<(), String> {
    {
        // Pre-allocate so concurrent writes to non-overlapping regions are
        // safe and the OS does not need to extend the file mid-download.
        let f = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| e.to_string())?;
        f.set_len(total).await.map_err(|e| e.to_string())?;
    }

    let progress_task = spawn_progress_reporter(
        app.clone(),
        downloaded.clone(),
        cancel.clone(),
        model_id.clone(),
        filename.clone(),
        total,
    );

    let chunk_size = (total + chunks - 1) / chunks;
    let mut join_set = JoinSet::new();

    for i in 0..chunks {
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size - 1).min(total - 1);

        if start >= total {
            break;
        }

        let client = Arc::clone(&client);
        let url = url.clone();
        let downloaded = Arc::clone(&downloaded);
        let cancel = Arc::clone(&cancel);
        let dest = dest.clone();

        join_set.spawn(async move {
            let mut current_start = start;
            let mut attempts: u32 = 0;

            loop {
                if attempts > 0 {
                    let delay = 250 * (1u64 << attempts.min(4));
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }

                // ── Connect phase ────────────────────────────────────────────
                let resp = client
                    .get(&url)
                    .header("Range", format!("bytes={}-{}", current_start, end))
                    .send()
                    .await;

                let resp = match resp {
                    Ok(r) if r.status().is_success() => {
                        // `probe` validated the initial resolved URL; this
                        // catches a second redirect on the chunk GET itself.
                        if let Some(host) = r.url().host_str() {
                            if !is_allowed_host(host) {
                                return Err(format!(
                                    "Chunk {} redirected to unexpected host: {}",
                                    i, host
                                ));
                            }
                        }
                        r
                    }
                    Ok(r) => {
                        return Err(format!(
                            "Chunk {} got unexpected status {}",
                            i,
                            r.status()
                        ));
                    }
                    Err(_) => {
                        attempts += 1;
                        if attempts >= MAX_CONNECT_RETRIES {
                            return Err(format!(
                                "Chunk {} failed to connect after {} attempts",
                                i, attempts
                            ));
                        }
                        continue;
                    }
                };

                // ── Stream phase ─────────────────────────────────────────────
                //
                // Spawn a fresh writer from `current_start` for this attempt.
                // On retry, `current_start` reflects bytes already on disk so
                // the writer seeks past them without re-downloading.
                let (frame_tx, frame_rx) = mpsc::channel::<Bytes>(WRITER_CHANNEL_CAPACITY);
                let write_handle = spawn_chunk_writer(dest.clone(), current_start, frame_rx);

                let mut stream = resp.bytes_stream();
                let mut stream_error = false;

                loop {
                    // The stall timeout catches a live connection that sends
                    // no bytes — without it, retry logic would never fire.
                    let next = tokio::time::timeout(STALL_TIMEOUT, stream.next()).await;

                    let chunk_res = match next {
                        Ok(Some(r)) => r,
                        Ok(None) => break,
                        Err(_stall) => {
                            stream_error = true;
                            break;
                        }
                    };

                    if cancel.load(Ordering::Relaxed) {
                        // Drop the sender so the writer drains and exits cleanly.
                        drop(frame_tx);
                        let _ = write_handle.await;
                        return Ok(());
                    }

                    let chunk = match chunk_res {
                        Ok(c) => c,
                        Err(_) => {
                            stream_error = true;
                            break;
                        }
                    };

                    let len = chunk.len() as u64;
                    // `send().await` only suspends if the writer has fallen
                    // 32 frames behind — natural backpressure without blocking
                    // the OS thread that owns the TCP socket.
                    if frame_tx.send(chunk).await.is_err() {
                        return Err(format!("Chunk {} writer died unexpectedly", i));
                    }
                    current_start += len;
                    downloaded.fetch_add(len, Ordering::Relaxed);
                }

                drop(frame_tx);
                write_handle.await.map_err(|e| e.to_string())??;

                if stream_error {
                    attempts += 1;
                    if attempts >= MAX_CONNECT_RETRIES + MAX_STREAM_RETRIES {
                        return Err(format!(
                            "Chunk {} stream failed after {} retries (byte {})",
                            i, attempts, current_start
                        ));
                    }
                    continue;
                }

                return Ok::<(), String>(());
            }
        });
    }

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                cancel.store(true, Ordering::SeqCst);
                // `join_set` is dropped here; `JoinSet::drop` calls `abort_all`.
                progress_task.abort();
                return Err(e);
            }
            Err(join_err) => {
                cancel.store(true, Ordering::SeqCst);
                // `join_set` is dropped here; `JoinSet::drop` calls `abort_all`.
                progress_task.abort();
                return Err(join_err.to_string());
            }
        }
    }

    progress_task.abort();
    Ok(())
}

pub async fn download_stream(
    app: AppHandle,
    client: Arc<reqwest::Client>,
    url: String,
    dest: PathBuf,
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

    let progress_task = spawn_progress_reporter(
        app.clone(),
        downloaded.clone(),
        cancel.clone(),
        model_id,
        filename,
        total,
    );

    // Same async/blocking split as `download_parallel`: network receive is
    // async; file writes run in a dedicated blocking thread. `write_from=0`
    // creates the file fresh and writes from the start.
    let (frame_tx, frame_rx) = mpsc::channel::<Bytes>(WRITER_CHANNEL_CAPACITY);
    let write_handle = spawn_chunk_writer(dest, 0, frame_rx);

    let mut stream = resp.bytes_stream();

    loop {
        let next = tokio::time::timeout(STALL_TIMEOUT, stream.next()).await;

        let chunk = match next {
            Ok(Some(r)) => r,
            Ok(None) => break,
            Err(_stall) => {
                drop(frame_tx);
                let _ = write_handle.await;
                progress_task.abort();
                return Err("Download stalled: no data received for 30 seconds".to_string());
            }
        };

        if cancel.load(Ordering::Relaxed) {
            drop(frame_tx);
            let _ = write_handle.await;
            progress_task.abort();
            return Ok(());
        }

        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        if frame_tx.send(chunk).await.is_err() {
            progress_task.abort();
            return Err("Stream writer died unexpectedly".to_string());
        }
    }

    drop(frame_tx);
    write_handle.await.map_err(|e| e.to_string())??;
    progress_task.abort();
    Ok(())
}