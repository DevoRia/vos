extern crate alloc;

use alloc::string::String;
use core::fmt::Write;
use uefi::boot;
use uefi::mem::memory_map::MemoryMap;
use uefi_raw::table::boot::{MemoryType, PAGE_SIZE};

pub struct MemoryInfo {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub boot_services_bytes: u64,
    pub runtime_bytes: u64,
    pub entry_count: usize,
}

pub fn get_memory_info() -> MemoryInfo {
    let map = boot::memory_map(MemoryType::LOADER_DATA).expect("failed to get memory map");

    let mut total: u64 = 0;
    let mut free: u64 = 0;
    let mut boot_svc: u64 = 0;
    let mut runtime: u64 = 0;

    for desc in map.entries() {
        let bytes = desc.page_count * PAGE_SIZE as u64;
        total += bytes;

        match desc.ty {
            MemoryType::CONVENTIONAL => free += bytes,
            MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA => {
                boot_svc += bytes;
            }
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => {
                runtime += bytes;
            }
            _ => {}
        }
    }

    MemoryInfo {
        total_bytes: total,
        free_bytes: free,
        boot_services_bytes: boot_svc,
        runtime_bytes: runtime,
        entry_count: map.len(),
    }
}

pub fn format_memory_info(info: &MemoryInfo) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "Memory Map ({} entries):", info.entry_count);
    let _ = writeln!(
        s,
        "  Total:          {} MB",
        info.total_bytes / (1024 * 1024)
    );
    let _ = writeln!(
        s,
        "  Free:           {} MB",
        info.free_bytes / (1024 * 1024)
    );
    let _ = writeln!(
        s,
        "  Used:           {} MB",
        (info.total_bytes - info.free_bytes) / (1024 * 1024)
    );
    let _ = writeln!(
        s,
        "  Boot Services:  {} KB",
        info.boot_services_bytes / 1024
    );
    let _ = writeln!(s, "  Runtime:        {} KB", info.runtime_bytes / 1024);
    s
}
