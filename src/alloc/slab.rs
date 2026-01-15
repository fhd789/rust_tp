use core::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{null_mut, NonNull};

use crate::alloc::bump::BumpAllocator;

const PAGE_SIZE: usize = 4096;
const CACHE_SIZES: [usize; 8] = [8, 16, 32, 64, 128, 256, 512, 1024];

#[repr(C)]
struct FreeNode {
    next: Option<NonNull<FreeNode>>,
}

#[repr(C)]
struct SlabHeader {
    next: Option<NonNull<SlabHeader>>,
    freelist: Option<NonNull<FreeNode>>,
    in_use: usize,
    total: usize,
    object_size: usize,
}

struct Cache {
    size: usize,
    slabs: Option<NonNull<SlabHeader>>,
}

pub struct SlabAllocator {
    caches: [Cache; CACHE_SIZES.len()],
    page_bump: BumpAllocator,
}

impl SlabAllocator {
    pub const fn new() -> Self {
        const EMPTY: Cache = Cache { size: 0, slabs: None };
        Self {
            caches: [EMPTY; CACHE_SIZES.len()],
            page_bump: BumpAllocator::new(),
        }
    }

    /// # Safety
    /// The caller must ensure the heap region is mapped, exclusive to this
    /// allocator, and aligned to at least PAGE_SIZE bytes.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        for (cache, size) in self.caches.iter_mut().zip(CACHE_SIZES.iter()) {
            cache.size = *size;
            cache.slabs = None;
        }
        // SAFETY: The heap range is reserved for this allocator's page bumps.
        unsafe { self.page_bump.init(align_up(heap_start, PAGE_SIZE), heap_size) };
    }

    fn alloc_from_cache(&mut self, cache_index: usize) -> *mut u8 {
        let cache = &mut self.caches[cache_index];
        let object_size = cache.size;

        let mut current = cache.slabs;
        while let Some(header_ptr) = current {
            // SAFETY: `header_ptr` comes from the cache list and is valid.
            let header = unsafe { header_ptr.as_ref() };
            if header.freelist.is_some() {
                // SAFETY: The slab has a non-empty freelist.
                return unsafe { self.pop_freelist(header_ptr) };
            }
            current = header.next;
        }

        let slab_ptr = match self.allocate_slab(object_size) {
            Some(ptr) => ptr,
            None => return null_mut(),
        };

        // SAFETY: The slab pointer is newly allocated and unique.
        unsafe { self.push_slab(cache, slab_ptr) };
        // SAFETY: The slab freelist is populated during initialization.
        unsafe { self.pop_freelist(slab_ptr) }
    }

    fn allocate_slab(&mut self, object_size: usize) -> Option<NonNull<SlabHeader>> {
        let layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).ok()?;
        let page_ptr = self.page_bump.alloc(layout) as *mut u8;
        if page_ptr.is_null() {
            return None;
        }

        let header_ptr = page_ptr as *mut SlabHeader;
        // SAFETY: The slab header lives at the start of the slab page.
        let header = unsafe { &mut *header_ptr };
        header.next = None;
        header.in_use = 0;
        header.object_size = object_size;

        let header_size = mem::size_of::<SlabHeader>();
        let object_start = align_up(page_ptr as usize + header_size, object_size);
        let object_end = page_ptr as usize + PAGE_SIZE;
        let capacity = (object_end.saturating_sub(object_start)) / object_size;
        header.total = capacity;
        header.freelist = None;

        let mut current = object_start;
        for _ in 0..capacity {
            let node_ptr = current as *mut FreeNode;
            // SAFETY: The object region is inside the slab page.
            let node = unsafe { &mut *node_ptr };
            node.next = header.freelist;
            // SAFETY: node_ptr is derived from a valid object address.
            header.freelist = Some(unsafe { NonNull::new_unchecked(node_ptr) });
            current += object_size;
        }

        // SAFETY: header_ptr is non-null and points to a valid slab header.
        Some(unsafe { NonNull::new_unchecked(header_ptr) })
    }

    /// # Safety
    /// The caller must ensure the slab pointer is valid and unique for the cache.
    unsafe fn push_slab(&mut self, cache: &mut Cache, slab: NonNull<SlabHeader>) {
        let slab_ref = slab.as_ptr();
        (*slab_ref).next = cache.slabs;
        cache.slabs = Some(slab);
    }

    /// # Safety
    /// The caller must ensure the slab pointer is valid and that its freelist is
    /// non-empty.
    unsafe fn pop_freelist(&mut self, slab: NonNull<SlabHeader>) -> *mut u8 {
        let header = slab.as_ptr();
        let node = (*header)
            .freelist
            .expect("freelist expected")
            .as_ptr();
        (*header).freelist = (*node).next;
        (*header).in_use += 1;
        node as *mut u8
    }

    /// # Safety
    /// The caller must ensure the pointer was allocated from this allocator and
    /// that the provided layout matches the allocation request.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let (cache_index, object_size) = match cache_for(layout) {
            Some(data) => data,
            None => return,
        };

        let page_base = align_down(ptr as usize, PAGE_SIZE) as *mut SlabHeader;
        // SAFETY: page_base is the slab header for this allocation.
        let header = unsafe { &mut *page_base };

        debug_assert_eq!(header.object_size, object_size);

        #[cfg(feature = "trace")]
        unsafe {
            // SAFETY: freelist_contains requires a valid freelist.
            assert!(!freelist_contains(header.freelist, ptr as *mut FreeNode));
        }

        let node = ptr as *mut FreeNode;
        // SAFETY: ptr points to a valid object and can store freelist metadata.
        unsafe {
            (*node).next = header.freelist;
        }
        // SAFETY: node is a valid pointer within the slab.
        header.freelist = Some(unsafe { NonNull::new_unchecked(node) });
        header.in_use = header.in_use.saturating_sub(1);

        if header.in_use == 0 {
            self.trim_slab(cache_index, page_base);
        }
    }

    fn trim_slab(&mut self, cache_index: usize, slab_ptr: *mut SlabHeader) {
        let cache = &mut self.caches[cache_index];
        let mut current = &mut cache.slabs;
        while let Some(node) = *current {
            if node.as_ptr() == slab_ptr {
                unsafe {
                    *current = (*slab_ptr).next;
                }
                break;
            }
            unsafe {
                current = &mut (*node.as_ptr()).next;
            }
        }
    }
}

/// # Safety
/// The caller must ensure that the allocator is initialized before any calls to
/// alloc or dealloc and that dealloc layout matches the original allocation.
unsafe impl GlobalAlloc for SlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc requires internal mutability for this allocator.
        let mut allocator = unsafe { &mut *(self as *const _ as *mut SlabAllocator) };
        match cache_for(layout) {
            Some((cache_index, _size)) => allocator.alloc_from_cache(cache_index),
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: GlobalAlloc requires internal mutability for this allocator.
        let mut allocator = unsafe { &mut *(self as *const _ as *mut SlabAllocator) };
        // SAFETY: Caller ensures layout matches original allocation.
        unsafe { allocator.dealloc(ptr, layout) };
    }
}

fn cache_for(layout: Layout) -> Option<(usize, usize)> {
    let size = layout.size().max(layout.align());
    let size = align_up(size, layout.align());
    CACHE_SIZES
        .iter()
        .position(|&cache_size| cache_size >= size)
        .map(|index| (index, CACHE_SIZES[index]))
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

#[cfg(feature = "trace")]
/// # Safety
/// The caller must ensure the list is valid and acyclic.
unsafe fn freelist_contains(mut head: Option<NonNull<FreeNode>>, ptr: *mut FreeNode) -> bool {
    while let Some(node) = head {
        if node.as_ptr() == ptr {
            return true;
        }
        // SAFETY: node points to a valid FreeNode in the list.
        head = unsafe { (*node.as_ptr()).next };
    }
    false
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use core::alloc::Layout;
    use std::vec::Vec;

    fn init_allocator() -> SlabAllocator {
        let mut allocator = SlabAllocator::new();
        let heap_size = PAGE_SIZE * 32;
        let heap = vec![0u8; heap_size].into_boxed_slice();
        let heap_start = heap.as_ptr() as usize;
        core::mem::forget(heap);
        unsafe { allocator.init(heap_start, heap_size) };
        allocator
    }

    #[test]
    fn alloc_reuse_after_free() {
        let mut allocator = init_allocator();
        let layout = Layout::from_size_align(64, 8).unwrap();
        let first = unsafe { allocator.alloc(layout) };
        let second = unsafe { allocator.alloc(layout) };
        assert!(!first.is_null());
        assert!(!second.is_null());
        unsafe { allocator.dealloc(first, layout) };
        let third = unsafe { allocator.alloc(layout) };
        assert_eq!(third, first, "freelist reuse failed");
    }

    #[test]
    fn alignment_respected() {
        let mut allocator = init_allocator();
        let layout = Layout::from_size_align(128, 64).unwrap();
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!(ptr as usize % 64, 0);
    }

    #[test]
    fn stress_random_alloc_free() {
        let mut allocator = init_allocator();
        let mut rng = XorShift64::new(0x1234_5678_9abc_def0);
        let mut allocations = Vec::new();

        for _ in 0..500 {
            if rng.next() & 1 == 0 || allocations.is_empty() {
                let size = [8, 16, 32, 64, 128, 256, 512][(rng.next() % 7) as usize];
                let layout = Layout::from_size_align(size, 8).unwrap();
                let ptr = unsafe { allocator.alloc(layout) };
                assert!(!ptr.is_null());
                allocations.push((ptr, layout));
            } else {
                let idx = (rng.next() as usize) % allocations.len();
                let (ptr, layout) = allocations.swap_remove(idx);
                unsafe { allocator.dealloc(ptr, layout) };
            }
        }

        for (ptr, layout) in allocations {
            unsafe { allocator.dealloc(ptr, layout) };
        }
    }

    #[test]
    fn overflow_returns_null() {
        let mut allocator = init_allocator();
        let layout = Layout::from_size_align(2048, 8).unwrap();
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(ptr.is_null());
    }

    struct XorShift64 {
        state: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }
    }
}
