use crate::hardware::types::MemoryInfo;

pub fn get_memory_info(sys: &sysinfo::System) -> MemoryInfo {
    let total_mem = sys.total_memory() as f64 / 1_073_741_824.0;
    MemoryInfo { total_gb: total_mem }
}