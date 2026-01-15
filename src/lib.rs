#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

pub mod alloc;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;

pub fn init(boot_info: &'static bootloader_api::BootInfo) {
    // SAFETY: Called once during early boot before interrupts are enabled.
    unsafe { gdt::init() };
    interrupts::init_idt();
    // SAFETY: We are initializing PICs during early boot with interrupts off.
    unsafe { interrupts::init_pics() };
    x86_64::instructions::interrupts::enable();

    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset);
    // SAFETY: The bootloader provides a valid physical memory offset mapping.
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // SAFETY: The bootloader memory map is valid for the lifetime of the kernel.
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    alloc::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
    hlt_loop();
}

#[cfg(test)]
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("TEST PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC: {}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// # Safety
/// The caller must ensure that the QEMU isa-debug-exit device is configured at
/// I/O port 0xF4; otherwise this write has no effect or may trigger undefined
/// behavior on real hardware.
unsafe fn exit_qemu_raw(code: QemuExitCode) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0xF4);
    port.write(code as u32);
}

pub fn exit_qemu(code: QemuExitCode) {
    // SAFETY: This is only used when running under QEMU with the exit device.
    unsafe { exit_qemu_raw(code) };
}

#[cfg(test)]
#[test_case]
fn trivial_assertion() {
    serial_println!("trivial assertion...");
    assert_eq!(1, 1);
}

#[cfg(test)]
#[test_case]
fn qemu_smoke_alloc() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    let mut vec = Vec::new();
    for i in 0..32 {
        vec.push(i);
    }

    let boxed = Box::new(0xdeadbeef_u64);
    assert_eq!(*boxed, 0xdeadbeef_u64);
    assert_eq!(vec.len(), 32);
    serial_println!("OK");
}
