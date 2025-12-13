//! Directory entry parsing utilities.
use alloc::{format, string::{String, ToString}};
use core::convert::TryInto;

/// Raw directory entry extracted from a directory cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub attr: u8,
    pub cluster: u32,
    pub size: u32,
}

impl DirEntry {
    pub const ATTR_DIRECTORY: u8 = 0x10;
    pub const ATTR_LONG_NAME: u8 = 0x0F;

    /// Whether this entry denotes a directory.
    pub fn is_dir(&self) -> bool {
        self.attr & Self::ATTR_DIRECTORY != 0
    }
}

/// Public information returned by `ls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u32,
}

/// Convert a short (8.3) name into a printable string.
pub fn short_name(raw: &[u8]) -> String {
    let name = &raw[..8];
    let ext = &raw[8..11];
    let trim_space = |s: &[u8]| {
        let mut end = s.len();
        while end > 0 && s[end - 1] == b' ' {
            end -= 1;
        }
        String::from_utf8_lossy(&s[..end]).to_string()
    };
    let base = trim_space(name);
    let extension = trim_space(ext);
    if extension.is_empty() {
        base
    } else {
        format!("{}.{}", base, extension)
    }
}

/// Parse a 32-byte FAT directory entry. Returns `None` on deleted or padding entries.
pub fn parse_dir_entry(raw: &[u8]) -> Option<DirEntry> {
    if raw.len() < 32 {
        return None;
    }
    match raw[0] {
        0x00 => return None, // end of directory
        0xE5 => return None, // deleted
        _ => {}
    }
    let attr = raw[11];
    if attr == DirEntry::ATTR_LONG_NAME {
        return None;
    }
    if attr == 0x08 {
        return None; // volume label
    }
    let name = short_name(&raw[0..11]);
    let cluster_high = u16::from_le_bytes(raw[20..22].try_into().ok()?);
    let cluster_low = u16::from_le_bytes(raw[26..28].try_into().ok()?);
    let cluster = ((cluster_high as u32) << 16) | cluster_low as u32;
    let size = u32::from_le_bytes(raw[28..32].try_into().ok()?);
    Some(DirEntry {
        name,
        attr,
        cluster,
        size,
    })
}
