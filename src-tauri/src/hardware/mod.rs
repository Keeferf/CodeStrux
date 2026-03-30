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

/// Gets comprehensive hardware information including CPU, memory, and GPU details.
#[tauri::command]
pub async fn get_hardware_info() -> HardwareInfo {
    use sysinfo::System;
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu_info = cpu::get_cpu_info(&sys);
    let memory_info = memory::get_memory_info(&sys);
    let gpu_info = gpu::get_best_gpu().await;
    
    HardwareInfo {
        cpu: cpu_info,
        memory: memory_info,
        gpu: gpu_info,
    }
}