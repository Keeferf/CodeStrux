use sysinfo::System;
use wgpu::Instance;

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct CpuInfo {
    pub name: String,
    pub cores: usize,
    pub usage: f32,
}

#[derive(serde::Serialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub usage: f32,
}

#[derive(serde::Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_total_gb: f64,
    pub vram_used_gb: Option<f64>,  // None on non-Windows platforms
}

#[derive(serde::Serialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: Option<GpuInfo>,
}

// ── WMI VRAM usage (Windows only) ────────────────────────────────────────────

#[cfg(windows)]
fn query_vram_used_gb() -> Option<f64> {
    use wmi::WMIConnection;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename = "Win32_PerfFormattedData_GPUPerformanceCounters_GPULocalAdapterMemory")]
    struct GpuMemPerf {
        #[serde(rename = "LocalAdapterMemoryUsage")]
        local_adapter_memory_usage: u64, // KB
    }

    let wmi = WMIConnection::new().ok()?;
    let results: Vec<GpuMemPerf> = wmi.query().ok()?;

    // Sum across all adapter segments and convert KB → GB
    let total_kb: u64 = results.iter().map(|r| r.local_adapter_memory_usage).sum();
    Some(total_kb as f64 / (1024.0 * 1024.0))
}

#[cfg(not(windows))]
fn query_vram_used_gb() -> Option<f64> {
    None
}

// ── Tauri command ─────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_hardware_info() -> HardwareInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU
    let cpus = sys.cpus();
    let cpu_name = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    // Memory
    let total_mem = sys.total_memory() as f64;
    let used_mem = sys.used_memory() as f64;
    let gb = 1_073_741_824.0_f64;
    let mem_usage = if total_mem > 0.0 {
        (used_mem / total_mem * 100.0) as f32
    } else {
        0.0
    };

    // GPU — first adapter for name + total VRAM, WMI for live usage
    let instance = Instance::default();
    let gpu = instance
        .enumerate_adapters(wgpu::Backends::all())
        .await
        .into_iter()
        .next()
        .map(|a: wgpu::Adapter| {
            let info = a.get_info();
            let vram_total_gb = a.limits().max_buffer_size as f64 / gb;
            let vram_used_gb = query_vram_used_gb();
            GpuInfo {
                name: info.name,
                vram_total_gb,
                vram_used_gb,
            }
        });

    HardwareInfo {
        cpu: CpuInfo {
            name: cpu_name,
            cores: cpus.len(),
            usage: sys.global_cpu_usage(),
        },
        memory: MemoryInfo {
            total_gb: total_mem / gb,
            used_gb: used_mem / gb,
            usage: mem_usage,
        },
        gpu,
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_hardware_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}