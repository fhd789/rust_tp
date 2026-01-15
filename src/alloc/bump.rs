use core::alloc::Layout;
use core::ptr::null_mut;

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
        }
    }

    /// # Safety
    /// The caller must ensure the heap range is valid, mapped, and not used by
    /// other allocators.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > self.heap_end {
            return null_mut();
        }

        self.next = alloc_end;
        alloc_start as *mut u8
    }

    pub fn reset(&mut self) {
        self.next = self.heap_start;
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
