#![cfg_attr(not(test), no_std)]
//! Minimal read-only FAT32 implementation.
//! The crate is `no_std` and uses an abstract block device for sector access.

extern crate alloc;

pub mod blockdev;
pub mod bpb;
pub mod dir;
pub mod fat;
pub mod fs;
pub mod path;

pub use blockdev::{BlockDevice, BlockError, MemDevice};
pub use bpb::BiosParameterBlock;
pub use dir::{DirEntry, DirEntryInfo};
pub use fat::{ClusterValue, FatTable};
pub use fs::{Fat32, FsError};
