#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)] 

extern crate alloc;

pub mod arch;
pub mod fs;
pub mod gui;
pub mod memory;
pub mod shell;

#[cfg(target_arch = "x86_64")]
pub use arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
pub use arch::aarch64::*;

pub fn init() {
    #[cfg(target_arch = "x86_64")]
    arch::x86_64::init();
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        // Serial print not generic yet
        self();
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    for test in tests {
        test.run();
    }
    // exit qemu
}

pub fn test_panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test_panic_handler(info)
}
