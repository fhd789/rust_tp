//! Block device abstraction for sector-based storage.
use alloc::vec::Vec;

/// Errors returned by block devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    /// Provided buffer has an unexpected size.
    BadBuffer,
    /// Underlying device I/O failed.
    IoError,
}

/// Abstraction over a sector-addressable storage backend.
pub trait BlockDevice {
    /// Returns the sector size in bytes.
    fn sector_size(&self) -> usize;
    /// Read a sector at the given logical block address into `buf`.
    fn read_sector(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError>;
}

/// In-memory device useful for testing.
#[derive(Clone, Debug)]
pub struct MemDevice {
    sector_size: usize,
    data: Vec<u8>,
}

impl MemDevice {
    /// Create a new memory-backed device with the given sector size.
    /// The `data` length must be a multiple of `sector_size`.
    pub fn new(sector_size: usize, data: Vec<u8>) -> Result<Self, BlockError> {
        if data.len() % sector_size != 0 {
            return Err(BlockError::BadBuffer);
        }
        Ok(Self { sector_size, data })
    }

    /// Number of sectors stored in the device.
    pub fn sector_count(&self) -> u64 {
        (self.data.len() / self.sector_size) as u64
    }
}

impl BlockDevice for MemDevice {
    fn sector_size(&self) -> usize {
        self.sector_size
    }

    fn read_sector(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if buf.len() != self.sector_size {
            return Err(BlockError::BadBuffer);
        }
        let start = lba as usize * self.sector_size;
        let end = start + self.sector_size;
        if end > self.data.len() {
            return Err(BlockError::IoError);
        }
        buf.copy_from_slice(&self.data[start..end]);
        Ok(())
    }
}
