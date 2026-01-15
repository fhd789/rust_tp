#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![test_runner(kernel_rust_slab_fahed_masad::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo};
use kernel_rust_slab_fahed_masad::hlt_loop;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel_rust_slab_fahed_masad::init(boot_info);

    kernel_rust_slab_fahed_masad::serial_println!("kernel initialized");

    #[cfg(test)]
    test_main();

    hlt_loop();
}
