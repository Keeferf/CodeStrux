use serde::Serialize;

/// CPU model name and logical core count.
#[derive(Serialize)]
pub struct CpuInfo {
    /// Shortened, human-readable model name (trademark symbols and clock speed
    /// stripped).
    pub name: String,
    pub cores: usize,
}

/// Total system RAM.
#[derive(Serialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
}

/// Best available GPU and its total VRAM.
#[derive(Serialize)]
pub struct GpuInfo {
    /// Shortened, human-readable adapter name (sub-brand and laptop qualifier
    /// stripped).
    pub name: String,
    /// `None` on non-Windows platforms where VRAM cannot be queried reliably.
    pub vram_gb: Option<f64>,
}

/// Aggregated hardware snapshot returned to the frontend.
#[derive(Serialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    /// `None` when no discrete or integrated GPU is detected.
    pub gpu: Option<GpuInfo>,
}