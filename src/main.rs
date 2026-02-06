#![no_std]
#![no_main]

use uefi::prelude::*;
use uefi::Status;
use uefi::Handle;

#[no_mangle]
pub extern "C" fn efi_main(handle: Handle, system_table: *mut uefi_raw::table::system::SystemTable) -> Status {
    unsafe {
        uefi::boot::set_image_handle(handle);
        uefi::table::set_system_table(system_table);
    }
    
    // Initialize helpers (allocator, logger)
    if let Err(_) = uefi::helpers::init() {
        return Status::ABORTED;
    }
    
    log::info!("UEFI Boot Success (Manual Entry)!");

    vos::shell::run_shell();
}

#[global_allocator]
static ALLOCATOR: uefi::allocator::Allocator = uefi::allocator::Allocator;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
