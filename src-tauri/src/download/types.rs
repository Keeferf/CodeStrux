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

// ── Shared cancel flag ────────────────────────────────────────────────────────

pub struct DownloadState {
    pub cancel: Arc<AtomicBool>,
    /// Limits concurrent downloads to exactly one at a time.
    ///
    /// A Semaphore with 1 permit is the simplest correct primitive here.
    /// try_acquire_owned() is non-blocking: success means this caller owns the
    /// slot; TryAcquireError means one is already running. The returned
    /// OwnedSemaphorePermit releases the slot automatically when dropped —
    /// whether the download returns normally, errors, or panics.
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
    /// Acquires the download slot, resets the cancel flag, and returns both
    /// the cancel Arc and a permit that must be held for the download's
    /// lifetime. Returns an error immediately if a download is already running.
    ///
    /// The reset happens only after acquiring the slot so a concurrent start()
    /// cannot clear the cancel flag of a download still in flight.
    pub fn start(&self) -> Result<(Arc<AtomicBool>, OwnedSemaphorePermit), String> {
        let permit = Arc::clone(&self.active)
            .try_acquire_owned()
            .map_err(|_| "A download is already in progress".to_string())?;
        self.cancel.store(false, Ordering::SeqCst);
        Ok((Arc::clone(&self.cancel), permit))
    }

    /// Signals any in-progress download to stop.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

// ── Rolling-window speed tracker ─────────────────────────────────────────────
//
// Keeps a sliding window of (timestamp, bytes) samples. Each tick we record
// the current byte count, prune samples older than WINDOW_SECS, then compute
// speed as Δbytes / Δtime over the surviving window. This gives a responsive
// but stable estimate — instantaneous reads are too jittery, a full-download
// average is too slow to reflect CDN speed changes.

const SPEED_WINDOW_SECS: f64 = 4.0;

pub struct SpeedTracker {
    /// Ring of (Instant, cumulative_bytes) samples, oldest first.
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedTracker {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(32),
        }
    }

    /// Record a new sample and return the current speed in bytes/sec.
    /// Returns `None` until at least two samples span a non-zero time window.
    pub fn record(&mut self, bytes: u64) -> Option<f64> {
        let now = Instant::now();
        self.samples.push_back((now, bytes));

        // Prune samples that have fallen outside the rolling window.
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
            // Window too narrow for a stable estimate — skip this tick.
            return None;
        }

        let delta_bytes = bytes.saturating_sub(oldest_bytes) as f64;
        Some(delta_bytes / elapsed)
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
    /// Rolling-window bytes/sec estimate. `None` during the first few ticks
    /// while the window fills up, or when `total` is unknown.
    pub speed_bps: Option<f64>,
    /// Estimated seconds remaining based on current speed. `None` when speed
    /// or total is unknown.
    pub eta_secs: Option<f64>,
}