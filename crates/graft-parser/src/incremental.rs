//! Fast hash-based incremental change detection for codebase indexing.

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct HashIndex {
    /// File path -> 64-bit FNV-1a content hash
    pub hashes: HashMap<String, u64>,
}

impl HashIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fast 64-bit FNV-1a hash of file bytes (zero external dependencies)
    pub fn hash_bytes(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for &byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3_u64);
        }
        hash
    }

    /// Check if a file's content hash matches the stored index
    pub fn is_unchanged(&self, path: &str, current_hash: u64) -> bool {
        self.hashes.get(path).copied() == Some(current_hash)
    }

    /// Update or record the hash for a file
    pub fn update(&mut self, path: String, hash: u64) {
        self.hashes.insert(path, hash);
    }

    /// Remove a deleted file from the hash index
    pub fn remove(&mut self, path: &str) -> Option<u64> {
        self.hashes.remove(path)
    }

    /// Save hash index to disk as JSON
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load hash index from disk
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let index: Self = serde_json::from_str(&content)?;
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes_deterministic() {
        let data1 = b"pub fn calculate() -> i32 { 100 }";
        let data2 = b"pub fn calculate() -> i32 { 100 }";
        let data3 = b"pub fn calculate() -> i32 { 101 }";

        assert_eq!(HashIndex::hash_bytes(data1), HashIndex::hash_bytes(data2));
        assert_ne!(HashIndex::hash_bytes(data1), HashIndex::hash_bytes(data3));
    }

    #[test]
    fn test_hash_index_tracking() {
        let mut index = HashIndex::new();
        let path = "src/main.rs".to_string();
        let hash = HashIndex::hash_bytes(b"fn main() {}");

        index.update(path.clone(), hash);
        assert!(index.is_unchanged(&path, hash));
        assert!(!index.is_unchanged(&path, hash + 1));
        assert!(!index.is_unchanged("src/other.rs", hash));
    }
}
