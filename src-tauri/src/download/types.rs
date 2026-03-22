//! Shared state types and event payload structs for the download subsystem.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ── Download state ────────────────────────────────────────────────────────────

/// Tauri-managed state that enforces a single concurrent download and exposes
/// a cancellation flag.
pub struct DownloadState {
    pub cancel: Arc<AtomicBool>,
    // One-permit semaphore used as a mutex-like download slot.
    // `try_acquire_owned` is non-blocking: success grants the slot;
    // `TryAcquireError` means one is already running. The permit releases
    // the slot automatically on drop, even on panic.
    active: Arc<Semaphore>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            active: Arc::new(Semaphore::new(1)),
        }
    }
}

impl DownloadState {
    /// Acquires the download slot and resets the cancellation flag.
    ///
    /// The returned permit **must be held** for the entire download lifetime;
    /// dropping it releases the slot. The flag is reset only after acquiring
    /// the slot to prevent a racing `start()` call from clearing the flag of
    /// an in-flight download.
    ///
    /// # Errors
    ///
    /// Returns an error string if a download is already in progress.
    pub fn start(&self) -> Result<(Arc<AtomicBool>, OwnedSemaphorePermit), String> {
        let permit = Arc::clone(&self.active)
            .try_acquire_owned()
            .map_err(|_| "A download is already in progress".to_string())?;
        self.cancel.store(false, Ordering::SeqCst);
        Ok((Arc::clone(&self.cancel), permit))
    }

    /// Signals any in-progress download to stop at its next cancellation check.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

// ── Speed tracker ─────────────────────────────────────────────────────────────

// Instantaneous reads are too jittery; a full-download average is too slow to
// reflect CDN speed changes. Four seconds balances responsiveness and stability.
const SPEED_WINDOW_SECS: f64 = 4.0;

/// Estimates download speed from a sliding window of cumulative byte samples.
///
/// Each [`record`](SpeedTracker::record) call appends a `(timestamp, bytes)`
/// sample, prunes entries older than `SPEED_WINDOW_SECS`, then computes speed
/// as `Δbytes / Δtime` over the surviving window.
pub struct SpeedTracker {
    /// `(timestamp, cumulative_bytes)` samples, oldest first.
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedTracker {
    /// Creates a new tracker with pre-allocated sample storage.
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(32),
        }
    }

    /// Records `bytes` (cumulative total downloaded so far) and returns the
    /// current speed in bytes per second.
    ///
    /// Returns `None` until at least two samples span a window wider than
    /// 50 ms, to avoid noisy estimates on startup.
    pub fn record(&mut self, bytes: u64) -> Option<f64> {
        let now = Instant::now();
        self.samples.push_back((now, bytes));

        while self
            .samples
            .front()
            .map(|(t, _)| now.duration_since(*t).as_secs_f64() > SPEED_WINDOW_SECS)
            .unwrap_or(false)
        {
            self.samples.pop_front();
        }

        let (oldest_t, oldest_bytes) = *self.samples.front()?;
        let elapsed = now.duration_since(oldest_t).as_secs_f64();
        if elapsed < 0.05 {
            return None;
        }

        let delta_bytes = bytes.saturating_sub(oldest_bytes) as f64;
        Some(delta_bytes / elapsed)
    }
}

// ── Event payloads ────────────────────────────────────────────────────────────

/// Payload emitted as `download-progress` on each progress tick.
#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub model_id: String,
    pub filename: String,
    /// Bytes written to disk so far.
    pub downloaded: u64,
    /// Total expected bytes, or `0` if unknown (streaming fallback).
    pub total: u64,
    /// Completion percentage in `[0.0, 100.0]`, or `0.0` when `total` is unknown.
    pub percent: f64,
    /// Rolling-window bytes/sec estimate. `None` during the first few ticks
    /// while the window fills, or when `total` is unknown.
    pub speed_bps: Option<f64>,
    /// Estimated seconds remaining. `None` when speed or `total` is unknown.
    pub eta_secs: Option<f64>,
}