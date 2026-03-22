//! Hardware introspection — CPU, memory, and GPU detection.
//!
//! CPU and memory are queried via [`sysinfo`]. GPU enumeration uses [`wgpu`]
//! for cross-platform adapter discovery; VRAM is read from the Windows registry
//! on Windows and left as `None` on other platforms.

use sysinfo::System;
use wgpu::Instance;

// ── Structs ───────────────────────────────────────────────────────────────────

/// CPU model name and logical core count.
#[derive(serde::Serialize)]
pub struct CpuInfo {
    /// Shortened, human-readable model name (trademark symbols and clock speed
    /// stripped).
    pub name: String,
    pub cores: usize,
}

/// Total system RAM.
#[derive(serde::Serialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
}

/// Best available GPU and its total VRAM.
#[derive(serde::Serialize)]
pub struct GpuInfo {
    /// Shortened, human-readable adapter name (sub-brand and laptop qualifier
    /// stripped).
    pub name: String,
    /// `None` on non-Windows platforms where VRAM cannot be queried reliably.
    pub vram_gb: Option<f64>,
}

/// Aggregated hardware snapshot returned to the frontend.
#[derive(serde::Serialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    /// `None` when no discrete or integrated GPU is detected.
    pub gpu: Option<GpuInfo>,
}

// ── Name helpers ──────────────────────────────────────────────────────────────

/// "Intel(R) Core(TM) i9-10900K CPU @ 3.70GHz" → "Intel i9-10900K"
/// "AMD Ryzen 9 5900X 12-Core Processor"         → "AMD Ryzen 9 5900X"
fn shorten_cpu_name(raw: &str) -> String {
    let s = raw
        .replace("(R)", "")
        .replace("(TM)", "");

    // Strip ordinal generation prefix: "12th Gen", "13th Gen", "3rd Gen", etc.
    let words: Vec<&str> = s.split_whitespace().collect();
    let s = match words.as_slice() {
        [gen, tag, rest @ ..] if
            tag.eq_ignore_ascii_case("gen") &&
            (gen.ends_with("th") || gen.ends_with("st") || gen.ends_with("nd") || gen.ends_with("rd")) =>
            rest.join(" "),
        _ => words.join(" "),
    };

    let s = s
        .find(" CPU @")
        .or_else(|| s.find(" @"))
        .map(|i| s[..i].to_string())
        .unwrap_or(s);

    let noise = ["Processor", "CPU"];
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| !noise.contains(w) && !w.ends_with("-Core") && !w.ends_with("-core"))
        .collect();

    // Intel CPUs include the redundant word "Core" between the brand and model
    // number (e.g. "Intel Core i9"); strip it for a cleaner display name.
    let words = if words.first().copied() == Some("Intel") {
        words.into_iter().filter(|w| *w != "Core").collect::<Vec<_>>()
    } else {
        words
    };

    let result = words.join(" ");
    if result.is_empty() { raw.trim().to_string() } else { result }
}

/// "NVIDIA GeForce RTX 4090" → "NVIDIA RTX 4090"
/// "AMD Radeon RX 7900 XTX"  → "AMD RX 7900 XTX"
fn shorten_gpu_name(raw: &str) -> String {
    let s = raw
        .replace("(R)", "")
        .replace("(TM)", "");
    // Laptop qualifiers must be stripped before prefix matching so that
    // e.g. "NVIDIA GeForce RTX 4060 Laptop GPU" still hits the prefix table.
    let mut s = s.trim().to_string();
    for phrase in &["Laptop GPU", "Laptop"] {
        if let Some(stripped) = s.trim_end().strip_suffix(phrase) {
            s = stripped.trim_end().to_string();
        }
    }
    let s = s.as_str();

    let sub_brands = [
        ("NVIDIA GeForce ", "NVIDIA "),
        ("AMD Radeon ",     "AMD "),
    ];
    for (full, short) in &sub_brands {
        if let Some(rest) = s.strip_prefix(full) {
            return format!("{}{}", short, rest.trim());
        }
    }

    s.to_string()
}

// ── VRAM total (Windows only) ─────────────────────────────────────────────────

// wgpu's `limits().max_buffer_size` is not a reliable VRAM proxy, so on
// Windows we read the authoritative value directly from the driver registry key.
#[cfg(windows)]
fn query_vram_total_gb() -> Option<f64> {
    use winreg::{enums::*, RegKey};
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let class = hklm
        .open_subkey(
            r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
        )
        .ok()?;
    class
        .enum_keys()
        .filter_map(|k| k.ok())
        .filter_map(|name| class.open_subkey(&name).ok())
        .find_map(|subkey| {
            let raw = subkey
                .get_raw_value("HardwareInformation.qwMemorySize")
                .ok()?;
            let bytes: [u8; 8] = raw.bytes.as_ref().try_into().ok()?;
            let vram_bytes = u64::from_le_bytes(bytes);
            (vram_bytes > 0).then(|| vram_bytes as f64 / 1_073_741_824.0)
        })
}

#[cfg(not(windows))]
fn query_vram_total_gb() -> Option<f64> {
    None
}

// ── Command ───────────────────────────────────────────────────────────────────

/// Returns a snapshot of the host CPU, memory, and best available GPU.
///
/// GPU selection prefers discrete over integrated adapters; among equal types
/// the adapter with the most VRAM wins, so a dedicated laptop GPU beats the
/// iGPU. CPU and software-only adapters are excluded entirely.
#[tauri::command]
pub async fn get_hardware_info() -> HardwareInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpus = sys.cpus();
    let raw_cpu_name = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let total_mem = sys.total_memory() as f64 / 1_073_741_824.0;

    let gpu = {
        let adapters = Instance::default()
            .enumerate_adapters(wgpu::Backends::all())
            .await;

        // Queried once here rather than once per adapter inside the iterator.
        let vram_gb = query_vram_total_gb();

        adapters
            .into_iter()
            .filter(|a| {
                !matches!(
                    a.get_info().device_type,
                    wgpu::DeviceType::Cpu | wgpu::DeviceType::Other
                )
            })
            .max_by_key(|a| {
                let type_score = match a.get_info().device_type {
                    wgpu::DeviceType::DiscreteGpu   => 2u64 << 32,
                    wgpu::DeviceType::IntegratedGpu => 1u64 << 32,
                    _                               => 0,
                };
                let vram_score = vram_gb.map_or(0, |gb| gb as u64);
                type_score + vram_score
            })
            .map(|adapter| GpuInfo {
                name: shorten_gpu_name(&adapter.get_info().name),
                vram_gb,
            })
    };

    HardwareInfo {
        cpu: CpuInfo {
            name: shorten_cpu_name(&raw_cpu_name),
            cores: cpus.len(),
        },
        memory: MemoryInfo {
            total_gb: total_mem,
        },
        gpu,
    }
}