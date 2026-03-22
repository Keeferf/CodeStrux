use std::time::Duration;

pub const ALLOWED_HOST: &str = "hf.co";
// Dot-prefixed form used by `is_allowed_host` to validate genuine subdomains.
const ALLOWED_HOST_SUFFIX: &str = ".hf.co";

/// Maximum number of parallel TCP connections used for chunked downloads.
pub const MAX_PARALLEL_CHUNKS: u64 = 16;

/// Target byte size per chunk for parallel downloads.
pub const TARGET_CHUNK_SIZE: u64 = 256 * 1024 * 1024;

/// Maximum accepted file size in bytes (100 GB).
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024 * 1024;

/// Returns `true` if `host` is exactly [`ALLOWED_HOST`] or a genuine subdomain.
pub fn is_allowed_host(host: &str) -> bool {
    host == ALLOWED_HOST || host.ends_with(ALLOWED_HOST_SUFFIX)
}

/// Builds an HTTP client configured for binary file downloads from HuggingFace.
///
/// Disables transparent decompression, forces HTTP/1.1 so each chunk gets its
/// own TCP connection (H2 multiplexing would defeat parallel downloading), and
/// attaches an optional Bearer token.
///
/// # Errors
///
/// Returns an error if the token contains characters invalid for an HTTP header
/// value, or if the underlying client cannot be constructed.
pub fn make_client(token: Option<&str>) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    // Guards against future middleware re-enabling transparent decompression,
    // which would corrupt binary transfers and break Content-Range accounting.
    headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        "identity".parse().unwrap(),
    );

    if let Some(t) = token {
        let value = format!("Bearer {}", t)
            .parse()
            .map_err(|_| "HuggingFace token contains invalid characters".to_string())?;
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .pool_max_idle_per_host(MAX_PARALLEL_CHUNKS as usize)
        .tcp_keepalive(Duration::from_secs(60))
        // No `.timeout()`: a wall-clock limit would kill large transfers.
        // Stall resilience is handled per-chunk in transfer.rs instead.
        .connect_timeout(Duration::from_secs(15))
        .http1_only()
        .build()
        .map_err(|e| e.to_string())
}

/// Resolves the final URL, total file size, and byte-range support for `url`.
///
/// Issues a `HEAD` request first; falls back to a single-byte range `GET`
/// when `HEAD` returns no usable `Content-Length`. The range response carries
/// an authoritative `Content-Range: bytes 0-0/<total>` header.
///
/// # Errors
///
/// Returns an error if the server responds with a non-success status, the
/// redirect target is not on an allowed host, or the reported size exceeds
/// [`MAX_FILE_BYTES`].
pub async fn probe(client: &reqwest::Client, url: &str) -> Result<(String, u64, bool), String> {
    // ── Step 1: HEAD ─────────────────────────────────────────────────────────
    let head = client
        .head(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !head.status().is_success() {
        return Err(format!("Probe failed: HTTP {}", head.status()));
    }

    let resolved = head.url().to_string();
    if let Some(host) = head.url().host_str() {
        if !is_allowed_host(host) {
            return Err(format!("Redirected to unexpected host: {}", host));
        }
    }

    let accepts_ranges = head
        .headers()
        .get("accept-ranges")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("bytes"))
        .unwrap_or(false);

    if let Some(total) = head.content_length().filter(|&n| n > 0) {
        if total > MAX_FILE_BYTES {
            return Err(format!("File too large: {} bytes exceeds {}-byte limit", total, MAX_FILE_BYTES));
        }
        return Ok((resolved, total, accepts_ranges));
    }

    // ── Step 2: Range GET fallback ────────────────────────────────────────────
    // Issue against the resolved URL to avoid re-following the redirect.
    let range_resp = client
        .get(&resolved)
        .header("Range", "bytes=0-0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if range_resp.status().as_u16() != 206 {
        // Range not supported — return total=0 so the caller falls back to
        // `download_stream`, which does not require a known size.
        return Ok((resolved, 0, false));
    }

    let total = range_resp
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').last())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if total > MAX_FILE_BYTES {
        return Err(format!("File too large: {} bytes exceeds {}-byte limit", total, MAX_FILE_BYTES));
    }

    // A 206 response is authoritative proof of range support even when HEAD
    // omitted `Accept-Ranges: bytes`.
    Ok((resolved, total, true))
}

/// Returns the number of parallel chunks for a file of `total` bytes,
/// clamped to `[1, `[`MAX_PARALLEL_CHUNKS`]`]`.
pub fn choose_chunks(total: u64) -> u64 {
    if total == 0 {
        return 1;
    }
    let chunks = (total + TARGET_CHUNK_SIZE - 1) / TARGET_CHUNK_SIZE;
    chunks.clamp(1, MAX_PARALLEL_CHUNKS)
}