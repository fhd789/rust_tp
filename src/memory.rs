use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

/// # Safety
/// The caller must guarantee that the provided physical memory offset correctly
/// maps physical addresses into the virtual address space and that the active
/// level 4 page table is accessible through that mapping.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// # Safety
/// The caller must ensure the physical memory offset is valid and that the
/// returned mutable reference is unique for the active level 4 page table.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    unsafe { &mut *page_table_ptr }
}

pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// # Safety
    /// The caller must ensure that the passed memory regions represent the
    /// system's memory map and remain valid for the lifetime of the allocator.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        Self { memory_regions, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        let regions = self.memory_regions.iter();
        let usable_regions = regions.filter(|region| region.kind == MemoryRegionKind::Usable);
        let addr_ranges = usable_regions.map(|region| region.start..region.end);
        let frame_addresses = addr_ranges.flat_map(|range| range.step_by(Size4KiB::SIZE as usize));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

/// # Safety
/// The allocator must only return usable frames and must never hand out the
/// same frame twice.
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
