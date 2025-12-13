//! High level FAT32 filesystem access.
use alloc::{format, string::{String, ToString}, vec, vec::Vec};

use crate::{
    blockdev::{BlockDevice, BlockError},
    bpb::BiosParameterBlock,
    dir::{parse_dir_entry, DirEntry, DirEntryInfo},
    fat::FatTable,
    path::normalize_path,
};

/// Errors returned by the filesystem API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    Block(BlockError),
    InvalidBpb,
    NotFound,
    NotADirectory,
    NotAFile,
}

impl From<BlockError> for FsError {
    fn from(e: BlockError) -> Self {
        FsError::Block(e)
    }
}

/// Read-only FAT32 filesystem instance.
#[derive(Debug, Clone)]
pub struct Fat32<D: BlockDevice> {
    device: D,
    pub bpb: BiosParameterBlock,
    lba_start: u64,
    fat: FatTable<D>,
    current_dir: u32,
    cwd_components: Vec<String>,
}

impl<D: BlockDevice + Clone> Fat32<D> {
    /// Mount a FAT32 volume at `lba_start` on the provided block device.
    pub fn mount(device: D, lba_start: u64) -> Result<Self, FsError> {
        let sector_size = device.sector_size();
        let mut buf = vec![0u8; sector_size];
        device.read_sector(lba_start, &mut buf)?;
        let bpb = BiosParameterBlock::parse(&buf).map_err(|_| FsError::InvalidBpb)?;
        let fat = FatTable {
            device: device.clone(),
            bpb: bpb.clone(),
            lba_start,
        };
        Ok(Self {
            device,
            bpb,
            lba_start,
            fat,
            current_dir: 0, // will be set to root
            cwd_components: Vec::new(),
        }
        .with_root())
    }

    fn with_root(mut self) -> Self {
        self.current_dir = self.bpb.root_cluster;
        self
    }

    /// Return the current working directory as a string.
    pub fn pwd(&self) -> String {
        if self.cwd_components.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.cwd_components.join("/"))
        }
    }

    /// Change the current directory.
    pub fn cd(&mut self, path: &str) -> Result<(), FsError> {
        let (is_abs, components) = normalize_path(path);
        let start_cluster = if is_abs { self.bpb.root_cluster } else { self.current_dir };
        let mut current = start_cluster;
        let mut new_components = if is_abs { Vec::new() } else { self.cwd_components.clone() };
        for part in components {
            let entry = self.find_in_dir(current, &part)?;
            if !entry.is_dir() {
                return Err(FsError::NotADirectory);
            }
            current = entry.cluster;
            new_components.push(entry.name);
        }
        self.current_dir = current;
        self.cwd_components = new_components;
        Ok(())
    }

    /// List directory contents for the given path or current directory.
    pub fn ls(&self, path: Option<&str>) -> Result<Vec<DirEntryInfo>, FsError> {
        let cluster = match path {
            Some(p) => self.resolve_path(p)?.cluster,
            None => self.current_dir,
        };
        let entries = self.read_directory(cluster)?;
        Ok(entries
            .into_iter()
            .map(|e| {
                let is_dir = e.is_dir();
                DirEntryInfo {
                    name: e.name,
                    is_dir,
                    size: e.size,
                }
            })
            .collect())
    }

    /// Read a file fully into memory.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let entry = self.resolve_path(path)?;
        if entry.is_dir() {
            return Err(FsError::NotAFile);
        }
        self.read_file_clusters(entry.cluster, entry.size)
    }

    fn resolve_path(&self, path: &str) -> Result<DirEntry, FsError> {
        let (is_abs, components) = normalize_path(path);
        let mut current = if is_abs { self.bpb.root_cluster } else { self.current_dir };
        let mut last_entry: Option<DirEntry> = None;
        for part in components.iter() {
            let entry = self.find_in_dir(current, part)?;
            current = entry.cluster;
            last_entry = Some(entry);
        }
        if components.is_empty() {
            // path refers to directory itself
            Ok(DirEntry {
                name: if is_abs { String::from("/") } else { String::from(".") },
                attr: DirEntry::ATTR_DIRECTORY,
                cluster: current,
                size: 0,
            })
        } else {
            last_entry.ok_or(FsError::NotFound)
        }
    }

    fn find_in_dir(&self, cluster: u32, name: &str) -> Result<DirEntry, FsError> {
        let entries = self.read_directory(cluster)?;
        entries
            .into_iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .ok_or(FsError::NotFound)
    }

    fn read_directory(&self, start_cluster: u32) -> Result<Vec<DirEntry>, FsError> {
        let mut cluster = start_cluster;
        let mut entries = Vec::new();
        loop {
            let data = self.read_cluster(cluster)?;
            for chunk in data.chunks(32) {
                if let Some(entry) = parse_dir_entry(chunk) {
                    entries.push(entry);
                } else if chunk[0] == 0x00 {
                    return Ok(entries);
                }
            }
            match self.fat.next_cluster(cluster)? {
                Some(next) => cluster = next,
                None => break,
            }
        }
        Ok(entries)
    }

    fn read_cluster(&self, cluster: u32) -> Result<Vec<u8>, FsError> {
        let mut buf = vec![0u8; self.bpb.cluster_size()];
        let first_lba = self.lba_start + self.bpb.cluster_to_lba(cluster);
        let mut offset = 0;
        for i in 0..self.bpb.sectors_per_cluster as u64 {
            let sector_lba = first_lba + i;
            let sector_slice = &mut buf[offset..offset + self.device.sector_size()];
            self.device.read_sector(sector_lba, sector_slice)?;
            offset += self.device.sector_size();
        }
        Ok(buf)
    }

    fn read_file_clusters(&self, start_cluster: u32, size: u32) -> Result<Vec<u8>, FsError> {
        let mut cluster = start_cluster;
        let mut data = Vec::new();
        let mut remaining = size as usize;
        loop {
            let block = self.read_cluster(cluster)?;
            if remaining <= block.len() {
                data.extend_from_slice(&block[..remaining]);
                break;
            } else {
                data.extend_from_slice(&block);
                remaining -= block.len();
            }
            match self.fat.next_cluster(cluster)? {
                Some(next) => cluster = next,
                None => break,
            }
        }
        Ok(data)
    }
}
