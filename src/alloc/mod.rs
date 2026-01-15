use core::alloc::Layout;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

use crate::alloc::locked::Locked;
use crate::alloc::slab::SlabAllocator;

pub mod bump;
pub mod locked;
pub mod slab;

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

#[global_allocator]
static ALLOCATOR: Locked<SlabAllocator> = Locked::new(SlabAllocator::new());

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<Size4KiB>,
) -> Result<(), x86_64::structures::paging::mapper::MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(x86_64::structures::paging::mapper::MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        // SAFETY: We are mapping freshly allocated frames into an unused heap range.
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // SAFETY: The heap range is mapped and dedicated to this allocator.
    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) };
    Ok(())
}

/// # Safety
/// The allocator must be initialized before calling this function.
pub unsafe fn alloc_layout(layout: Layout) -> *mut u8 {
    unsafe { ALLOCATOR.alloc(layout) }
}
