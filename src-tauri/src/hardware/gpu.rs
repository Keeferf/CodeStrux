use wgpu::Instance;
use crate::hardware::types::GpuInfo;

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

pub async fn get_best_gpu() -> Option<GpuInfo> {
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
}