//! BIOS Parameter Block parsing utilities.
use core::convert::TryInto;

use crate::blockdev::BlockError;

/// Minimal fields required from a FAT32 BIOS Parameter Block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiosParameterBlock {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub num_fats: u8,
    pub sectors_per_fat: u32,
    pub root_cluster: u32,
}

impl BiosParameterBlock {
    /// Parse the BPB from a boot sector.
    pub fn parse(sector: &[u8]) -> Result<Self, BlockError> {
        if sector.len() < 512 {
            return Err(BlockError::BadBuffer);
        }
        let bytes_per_sector = u16::from_le_bytes(sector[11..13].try_into().unwrap());
        let sectors_per_cluster = sector[13];
        let reserved_sector_count = u16::from_le_bytes(sector[14..16].try_into().unwrap());
        let num_fats = sector[16];
        let sectors_per_fat = u32::from_le_bytes(sector[36..40].try_into().unwrap());
        let root_cluster = u32::from_le_bytes(sector[44..48].try_into().unwrap());
        Ok(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sector_count,
            num_fats,
            sectors_per_fat,
            root_cluster,
        })
    }

    /// Number of bytes per cluster.
    pub fn cluster_size(&self) -> usize {
        self.bytes_per_sector as usize * self.sectors_per_cluster as usize
    }

    /// First data sector relative to the start of the volume (LBA based).
    pub fn first_data_sector(&self) -> u64 {
        (self.reserved_sector_count as u64) +
            (self.num_fats as u64 * self.sectors_per_fat as u64)
    }

    /// Convert a cluster index to the LBA of its first sector.
    pub fn cluster_to_lba(&self, cluster: u32) -> u64 {
        self.first_data_sector() + (cluster as u64 - 2) * self.sectors_per_cluster as u64
    }
}
