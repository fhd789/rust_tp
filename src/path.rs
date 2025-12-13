//! Path parsing helpers for FAT32 volumes.
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Break a path into normalized components.
/// Supports absolute (starting with `/`) and relative paths.
pub fn normalize_path(path: &str) -> (bool, Vec<String>) {
    let is_abs = path.starts_with('/');
    let mut components = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if !components.is_empty() {
                components.pop();
            }
            continue;
        }
        components.push(part.to_string());
    }
    (is_abs, components)
}
