//! FAT table traversal utilities.
use alloc::vec;

use crate::{blockdev::{BlockDevice, BlockError}, bpb::BiosParameterBlock};

/// Cluster values returned when following the FAT.
pub type ClusterValue = u32;

/// Minimum value considered end-of-chain for FAT32.
const EOC: u32 = 0x0FFFFFF8;

/// Access to the FAT table.
#[derive(Debug, Clone)]
pub struct FatTable<D: BlockDevice> {
    pub device: D,
    pub bpb: BiosParameterBlock,
    pub lba_start: u64,
}

impl<D: BlockDevice> FatTable<D> {
    /// Read the next cluster value from the FAT for `cluster`.
    pub fn next_cluster(&self, cluster: u32) -> Result<Option<u32>, BlockError> {
        let fat_offset = cluster as u64 * 4;
        let sector = self.bpb.reserved_sector_count as u64 + fat_offset / self.bpb.bytes_per_sector as u64;
        let offset = (fat_offset % self.bpb.bytes_per_sector as u64) as usize;
        let mut buf = vec![0u8; self.device.sector_size()];
        self.device.read_sector(self.lba_start + sector, &mut buf)?;
        let entry_bytes = &buf[offset..offset + 4];
        let value = u32::from_le_bytes(entry_bytes.try_into().unwrap()) & 0x0FFFFFFF;
        if value >= EOC {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }
}
