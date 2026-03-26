//! Hardware introspection — CPU, memory, and GPU detection.
//!
//! CPU and memory are queried via [`sysinfo`]. GPU enumeration uses [`wgpu`]
//! for cross-platform adapter discovery; VRAM is read from the Windows registry
//! on Windows and left as `None` on other platforms.

mod cpu;
mod memory;
mod gpu;
mod types;

pub use types::HardwareInfo;

/// Returns a snapshot of the host CPU, memory, and best available GPU.
///
/// GPU selection prefers discrete over integrated adapters; among equal types
/// the adapter with the most VRAM wins, so a dedicated laptop GPU beats the
/// iGPU. CPU and software-only adapters are excluded entirely.
#[tauri::command]
pub async fn get_hardware_info() -> HardwareInfo {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    HardwareInfo {
        cpu: cpu::get_cpu_info(&sys),
        memory: memory::get_memory_info(&sys),
        gpu: gpu::get_best_gpu().await,
    }
}