use std::time::Duration;

pub const ALLOWED_HOST: &str = "hf.co";
const ALLOWED_HOST_SUFFIX: &str = ".hf.co";
// Ceiling for parallel TCP connections to HuggingFace's Cloudflare CDN.
//
// 8 was the original measured sweet spot, but CDN behaviour varies by region
// and file popularity.  16 is safe to try — if the CDN throttles you, the
// per-chunk retry logic absorbs the 429/503 without losing progress.
// Raising above 24 gives diminishing returns: your NIC/ISP upstream becomes
// the bottleneck before the CDN does.
//
// To tune: run the same 7GB download at 8, 12, 16 and compare wall-clock
// times.  Pick the highest that doesn't trigger CDN rate-limit errors.
pub const MAX_PARALLEL_CHUNKS: u64 = 16;
// 256MB per chunk gives each TCP connection a long-lived, high-throughput
// session rather than many short ones with high setup overhead.
// At 16 chunks this covers files up to 4GB before chunk count saturates;
// larger files get more chunks up to the ceiling above.
pub const TARGET_CHUNK_SIZE: u64 = 256 * 1024 * 1024;
// Hard cap on claimed file size. Rejects a server lying about Content-Length
// or Content-Range before we pre-allocate disk space or begin streaming.
// 100 GB is well above any current LLM weight file.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024 * 1024;

/// Returns true if `host` is exactly the allowed host or a subdomain of it.
///
/// `ends_with(ALLOWED_HOST)` alone is NOT sufficient: `"evilhf.co"` satisfies
/// `ends_with("hf.co")`. We require either an exact match or a dot-prefixed
/// suffix so that only genuine subdomains pass (e.g. `cdn-lfs-us-1.hf.co`).
pub fn is_allowed_host(host: &str) -> bool {
    host == ALLOWED_HOST || host.ends_with(ALLOWED_HOST_SUFFIX)
}

pub fn make_client(token: Option<&str>) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    // Explicitly opt out of content encoding. Without gzip/brotli features
    // reqwest won't advertise them, but this header guards against any future
    // middleware re-enabling transparent decompression, which would corrupt
    // binary file transfers and break Content-Range accounting.
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
        // No overall timeout: .timeout() covers the entire streaming body read.
        // A 7GB model at 20MB/s takes ~350s — any wall-clock limit kills it.
        // Stall resilience is handled by per-chunk stall timeout in transfer.rs.
        .connect_timeout(Duration::from_secs(15))
        // Force HTTP/1.1 so each chunk gets its own TCP connection.
        // Cloudflare supports H2; if negotiated, all range requests would be
        // multiplexed over one TCP stream, defeating parallel downloading.
        .http1_only()
        .build()
        .map_err(|e| e.to_string())
}

/// Probes the URL to resolve the final redirected URL, total file size,
/// and whether the server supports byte-range requests.
///
/// Strategy — HEAD first, Range GET fallback:
///
///   1. HEAD: zero-body, fast. Sufficient when the CDN returns Content-Length.
///      Cloudflare sometimes omits Content-Length on HEAD responses to
///      redirected blob-storage targets, so we can't rely on it alone.
///
///   2. Range GET (bytes=0-0): fired only when HEAD returns no usable size.
///      A 206 response always carries `Content-Range: bytes 0-0/<total>`,
///      which is the authoritative file size. Only one body byte is transferred.
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

    // Validate the redirect destination is still on the allowed host.
    // is_allowed_host() requires an exact match or a genuine subdomain —
    // a plain ends_with check would accept any domain ending in "hf.co".
    let resolved = head.url().to_string();
    if let Some(host) = head.url().host_str() {
        if !is_allowed_host(host) {
            return Err(format!("Redirected to unexpected host: {}", host));
        }
    }

    // Accept-Ranges: bytes — read from HEAD, authoritative regardless of path.
    let accepts_ranges = head
        .headers()
        .get("accept-ranges")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("bytes"))
        .unwrap_or(false);

    // If HEAD gave us a non-zero Content-Length we're done — zero body cost.
    if let Some(total) = head.content_length().filter(|&n| n > 0) {
        if total > MAX_FILE_BYTES {
            return Err(format!("File too large: {} bytes exceeds {}-byte limit", total, MAX_FILE_BYTES));
        }
        return Ok((resolved, total, accepts_ranges));
    }

    // ── Step 2: Range GET fallback ────────────────────────────────────────────
    // Issue against the already-resolved URL so we don't re-follow the redirect.
    let range_resp = client
        .get(&resolved)
        .header("Range", "bytes=0-0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if range_resp.status().as_u16() != 206 {
        // Range not supported — return total=0 so commands.rs falls back to
        // download_stream, which doesn't need a known size.
        return Ok((resolved, 0, false));
    }

    let total = range_resp
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        // Format: "bytes 0-0/<total>"
        .and_then(|s| s.split('/').last())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if total > MAX_FILE_BYTES {
        return Err(format!("File too large: {} bytes exceeds {}-byte limit", total, MAX_FILE_BYTES));
    }

    // A 206 response is authoritative proof that ranges are supported,
    // even if the HEAD response didn't include Accept-Ranges: bytes.
    Ok((resolved, total, true))
}

/// Scales parallel chunk count based on an optimal target size.
pub fn choose_chunks(total: u64) -> u64 {
    if total == 0 {
        return 1;
    }
    let chunks = (total + TARGET_CHUNK_SIZE - 1) / TARGET_CHUNK_SIZE;
    chunks.clamp(1, MAX_PARALLEL_CHUNKS)
}