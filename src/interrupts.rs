use crate::{gdt, hlt_loop, serial_println};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        // SAFETY: The IST index points to a dedicated double-fault stack in the TSS.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}

lazy_static! {
    static ref PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
        // SAFETY: We expect legacy PICs at standard ports in this platform.
        create_pics()
    });
}

pub fn init_idt() {
    IDT.load();
}

/// # Safety
/// The caller must ensure the PICs exist at standard legacy ports and that no
/// other code is concurrently configuring them.
unsafe fn create_pics() -> ChainedPics {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
}

/// # Safety
/// Interrupts must be disabled while initializing the PICs to avoid races with
/// interrupt delivery.
pub unsafe fn init_pics() {
    PICS.lock().initialize();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Accessed Address (CR2): {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("RIP: {:?}", stack_frame.instruction_pointer);
    hlt_loop();
}
