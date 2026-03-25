//! Append-only diagnostic logging to a file on disk.

/// Appends `message` to the given log file, creating it if needed.
///
/// Failures are silently ignored — logging must never surface as a user-visible
/// error. The log file is the primary diagnostic tool since errors are
/// intentionally not shown in the UI.
pub fn write_log(log_path: &std::path::Path, message: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{}] {}", ts, message);
    }
}