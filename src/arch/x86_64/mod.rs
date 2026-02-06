// pub mod gdt;
// pub mod interrupts;
// pub mod memory;
// pub mod serial;
// pub mod vga_buffer;
// pub mod allocator; // Legacy allocator disabled for UEFI

pub fn init() {
    // gdt::init();
    // interrupts::init_idt();
    // unsafe { interrupts::PICS.lock().initialize() };
    // x86_64::instructions::interrupts::enable();
}
