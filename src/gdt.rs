use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // SAFETY: The double-fault stack is a static buffer reserved for the IST.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            unsafe { double_fault_stack_end() };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// # Safety
/// The caller must ensure the returned address points to a valid, static stack
/// that will not be concurrently mutated by other code.
unsafe fn double_fault_stack_end() -> VirtAddr {
    const STACK_SIZE: usize = 4096 * 5;
    static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let stack_start = VirtAddr::from_ptr(&STACK);
    stack_start + STACK_SIZE
}

/// # Safety
/// The caller must ensure this is invoked once during boot before enabling
/// interrupts, and that the GDT/TSS remain valid for the kernel lifetime.
pub unsafe fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    // SAFETY: The selectors were just loaded into the GDT and remain valid.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
