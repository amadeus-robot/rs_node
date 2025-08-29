use blake3;
use std::f64;

pub struct SolBloom;

impl SolBloom {
    pub const N: usize = 1_000_000;
    pub const K: usize = 2;

    pub const PAGES: usize = 256;
    pub const PAGE_SIZE: usize = 65_536; // 8kb
    pub const M: usize = Self::PAGES * Self::PAGE_SIZE; // 16,777,216 (2MB)

    /// Return total pages
    pub fn pages() -> usize {
        Self::PAGES
    }

    /// Return page size
    pub fn page_size() -> usize {
        Self::PAGE_SIZE
    }

    /// Return m
    pub fn m() -> usize {
        Self::M
    }

    /// n elements, m bits, k hashes
    /// Simulate False Positive Rate of Bloom filter
    pub fn simulate_fpr(n: usize, m: usize, k: usize) -> f64 {
        if n > 0 && m > 0 && k > 0 {
            (1.0 - f64::exp(-(k as f64) * (n as f64) / (m as f64))).powi(k as i32)
        } else {
            0.0
        }
    }

    /// Hash into bloom indices
    pub fn hash(bin: &[u8]) -> Vec<usize> {
        let digest = blake3::hash(bin);
        let mut indices = Vec::new();

        // Iterate over digest as 128-bit little-endian chunks
        let bytes = digest.as_bytes();
        for chunk in bytes.chunks_exact(16) {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(chunk);
            let word = u128::from_le_bytes(arr);
            indices.push((word % (Self::M as u128)) as usize);
        }

        indices
    }

    /// Segment into page + bit offset
    pub fn segs(bin: &[u8]) -> Vec<PageSegment> {
        let digest = blake3::hash(bin);
        let bytes = digest.as_bytes();
        let mut segments = Vec::new();

        for chunk in bytes.chunks_exact(16) {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(chunk);
            let word = u128::from_le_bytes(arr);
            let idx = (word % (Self::M as u128)) as usize;

            segments.push(PageSegment {
                page: idx / Self::PAGE_SIZE,
                bit_offset: idx % Self::PAGE_SIZE,
            });
        }

        segments
    }
}

/// Struct for segments
#[derive(Debug, Clone)]
pub struct PageSegment {
    pub page: usize,
    pub bit_offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpr() {
        let fpr = SolBloom::simulate_fpr(10_000_000, 64_000_000, 8);
        println!("FPR: {}", fpr);
        assert!(fpr > 0.0);
    }

    #[test]
    fn test_hash_and_segs() {
        let input = b"hello world";
        let indices = SolBloom::hash(input);
        let segs = SolBloom::segs(input);

        println!("Indices: {:?}", indices);
        println!("Segments: {:?}", segs);

        assert!(!indices.is_empty());
        assert_eq!(indices.len(), segs.len());
    }
}
