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
// If no bytes arrive within this window, the connection is considered stalled.
// Distinct from connect_timeout (15s): this fires on a live-but-silent
// connection, which would otherwise never trigger stream_error or retry logic.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
// Channel capacity per chunk: enough to buffer a few network frames ahead of
// the disk writer without holding too much data in memory.
const WRITER_CHANNEL_CAPACITY: usize = 32;
// 512KB write buffer: coalesces many small HTTP frames (typically 16–64KB)
// into fewer, larger syscalls.
const WRITE_BUF_SIZE: usize = 512 * 1024;

/// Spawns a background task that emits `download-progress` events on a fixed
/// interval until the download completes, is cancelled, or the handle is aborted.
///
/// Each tick records the current byte count into a rolling-window SpeedTracker
/// and derives bytes/sec + ETA before emitting. The tracker lives entirely
/// inside this task — no locking needed.
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

/// Spawns a blocking writer thread for one chunk attempt.
///
/// The writer opens the pre-allocated file with a plain std::fs::File, seeks
/// once to `write_from`, then drains the channel with blocking_recv() writing
/// each frame sequentially into a std::io::BufWriter.
///
/// Using std::fs/std::io here is deliberate: tokio::fs::File dispatches every
/// flush through spawn_blocking internally, which means each 512KB buffer fill
/// parks the async task waiting for a thread-pool slot. That parking suspends
/// the TCP receive loop and lets the OS socket buffer fill up, throttling the
/// sender. A single dedicated blocking thread per chunk has no such overhead —
/// disk writes run continuously without ever touching the async runtime.
fn spawn_chunk_writer(
    dest: PathBuf,
    write_from: u64,
    mut rx: mpsc::Receiver<Bytes>,
) -> tokio::task::JoinHandle<Result<(), String>> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .write(true)
            // create(true) without truncate(true): creates the file if it
            // doesn't exist (stream mode), opens without truncating if it does
            // (parallel mode, where the file is pre-allocated).
            .create(true)
            .open(&dest)
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::with_capacity(WRITE_BUF_SIZE, file);
        writer
            .seek(SeekFrom::Start(write_from))
            .map_err(|e| e.to_string())?;

        // blocking_recv() is safe inside spawn_blocking — it parks the OS
        // thread (not a tokio task) while waiting for the next frame.
        while let Some(data) = rx.blocking_recv() {
            writer.write_all(&data).map_err(|e| e.to_string())?;
        }
        // Channel closed (sender dropped) — flush remaining buffer to disk.
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
        // Pre-allocate the file so concurrent writes to non-overlapping
        // regions are safe and the OS doesn't need to extend the file
        // mid-download.
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
                        // Validate that any CDN redirect stayed on the allowed host.
                        // probe() validated the initial resolved URL; this catches
                        // a second redirect on the chunk GET itself.
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
                // Spawn a fresh blocking writer from current_start for this
                // attempt. On retry current_start reflects bytes already
                // committed to disk, so the new writer seeks past them.
                let (frame_tx, frame_rx) = mpsc::channel::<Bytes>(WRITER_CHANNEL_CAPACITY);
                let write_handle =
                    spawn_chunk_writer(dest.clone(), current_start, frame_rx);

                let mut stream = resp.bytes_stream();
                let mut stream_error = false;

                loop {
                    // Wrap stream.next() in a stall timeout. A connection that
                    // stays open but sends no bytes would otherwise never
                    // trigger stream_error — the retry logic would never fire.
                    let next = tokio::time::timeout(STALL_TIMEOUT, stream.next()).await;

                    let chunk_res = match next {
                        Ok(Some(r)) => r,
                        Ok(None) => break, // stream finished cleanly
                        Err(_stall) => {
                            stream_error = true;
                            break;
                        }
                    };

                    if cancel.load(Ordering::Relaxed) {
                        // Drop sender → writer drains remaining frames and exits.
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
                    // send().await only suspends if the blocking writer has
                    // fallen 32 frames behind — natural backpressure without
                    // ever blocking the OS thread that owns the TCP socket.
                    if frame_tx.send(chunk).await.is_err() {
                        return Err(format!("Chunk {} writer died unexpectedly", i));
                    }
                    current_start += len;
                    downloaded.fetch_add(len, Ordering::Relaxed);
                }

                // Close the channel so the writer knows to flush and exit.
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
                    // Next iteration spawns a fresh writer from current_start.
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
                join_set.abort_all();
                progress_task.abort();
                return Err(e);
            }
            Err(join_err) => {
                cancel.store(true, Ordering::SeqCst);
                join_set.abort_all();
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

    // Same async/blocking split as download_parallel: network receive is async,
    // file writes are in a dedicated blocking thread. spawn_chunk_writer with
    // write_from=0 creates the file fresh and writes from the beginning.
    let (frame_tx, frame_rx) = mpsc::channel::<Bytes>(WRITER_CHANNEL_CAPACITY);
    let write_handle = spawn_chunk_writer(dest, 0, frame_rx);

    let mut stream = resp.bytes_stream();

    loop {
        let next = tokio::time::timeout(STALL_TIMEOUT, stream.next()).await;

        let chunk = match next {
            Ok(Some(r)) => r,
            Ok(None) => break, // stream finished cleanly
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