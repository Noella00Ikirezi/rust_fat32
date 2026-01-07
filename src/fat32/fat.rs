//! FAT (File Allocation Table) handling
//!
//! The FAT is an array of 32-bit entries that form cluster chains.
//! Each entry points to the next cluster in a file or indicates end-of-chain.

extern crate alloc;
use alloc::vec::Vec;

/// FAT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatEntry {
    /// Cluster is free (0x00000000)
    Free,
    /// Reserved cluster (0x00000001)
    Reserved,
    /// Data cluster - value is next cluster number
    Data(u32),
    /// Bad cluster (0x0FFFFFF7)
    BadCluster,
    /// End of cluster chain (0x0FFFFFF8-0x0FFFFFFF)
    EndOfChain,
}

impl FatEntry {
    /// Parse raw 32-bit FAT entry value
    ///
    /// FAT32 uses only the lower 28 bits, upper 4 bits are reserved.
    ///
    /// # Arguments
    /// * `value` - Raw 32-bit value from FAT table
    pub fn from_raw(value: u32) -> Self {
        // Mask to 28 bits (FAT32 uses lower 28 bits only)
        match value & 0x0FFFFFFF {
            0x00000000 => FatEntry::Free,
            0x00000001 => FatEntry::Reserved,
            0x0FFFFFF7 => FatEntry::BadCluster,
            0x0FFFFFF8..=0x0FFFFFFF => FatEntry::EndOfChain,
            n => FatEntry::Data(n),
        }
    }

    /// Check if this entry marks end of chain
    #[inline]
    pub fn is_end(&self) -> bool {
        matches!(self, FatEntry::EndOfChain)
    }

    /// Check if this entry is free
    #[inline]
    pub fn is_free(&self) -> bool {
        matches!(self, FatEntry::Free)
    }

    /// Get next cluster number if this is a data entry
    #[inline]
    pub fn next_cluster(&self) -> Option<u32> {
        match self {
            FatEntry::Data(n) => Some(*n),
            _ => None,
        }
    }
}

/// FAT table reader
///
/// Provides read-only access to the File Allocation Table.
pub struct FatTable<'a> {
    /// Raw FAT data (array of 32-bit little-endian entries)
    data: &'a [u8],
}

impl<'a> FatTable<'a> {
    /// Create new FAT table reader
    ///
    /// # Arguments
    /// * `data` - Raw bytes of the FAT table
    pub fn new(data: &'a [u8]) -> Self {
        FatTable { data }
    }

    /// Get FAT entry for a cluster
    ///
    /// # Arguments
    /// * `cluster` - Cluster number to look up
    ///
    /// # Returns
    /// FAT entry for the cluster, or EndOfChain if out of bounds
    pub fn get_entry(&self, cluster: u32) -> FatEntry {
        let offset = (cluster as usize) * 4;
        if offset + 4 > self.data.len() {
            return FatEntry::EndOfChain;
        }

        let value = u32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]);

        FatEntry::from_raw(value)
    }

    /// Get complete cluster chain starting from a cluster
    ///
    /// Follows the FAT chain until end-of-chain marker is reached.
    ///
    /// # Arguments
    /// * `start` - Starting cluster number
    ///
    /// # Returns
    /// Vector of all cluster numbers in the chain
    pub fn get_cluster_chain(&self, start: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut current = start;

        // Maximum iterations to prevent infinite loops
        const MAX_CHAIN_LENGTH: usize = 1_000_000;

        loop {
            // Cluster numbers < 2 are invalid for data
            if current < 2 {
                break;
            }

            // Prevent infinite loops
            if chain.len() >= MAX_CHAIN_LENGTH {
                break;
            }

            chain.push(current);

            match self.get_entry(current) {
                FatEntry::Data(next) => {
                    // Detect simple cycles
                    if next == current {
                        break;
                    }
                    current = next;
                }
                _ => break,
            }
        }

        chain
    }

    /// Count free clusters in the FAT
    ///
    /// # Arguments
    /// * `total_clusters` - Total number of data clusters
    pub fn count_free_clusters(&self, total_clusters: u32) -> u32 {
        let mut count = 0;
        for cluster in 2..total_clusters + 2 {
            if self.get_entry(cluster).is_free() {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fat_entry_types() {
        assert_eq!(FatEntry::from_raw(0x00000000), FatEntry::Free);
        assert_eq!(FatEntry::from_raw(0x00000001), FatEntry::Reserved);
        assert_eq!(FatEntry::from_raw(0x00000064), FatEntry::Data(100));
        assert_eq!(FatEntry::from_raw(0x0FFFFFF7), FatEntry::BadCluster);
        assert_eq!(FatEntry::from_raw(0x0FFFFFF8), FatEntry::EndOfChain);
        assert_eq!(FatEntry::from_raw(0x0FFFFFFF), FatEntry::EndOfChain);
    }

    #[test]
    fn test_fat_entry_methods() {
        assert!(FatEntry::EndOfChain.is_end());
        assert!(!FatEntry::Data(5).is_end());
        assert!(FatEntry::Free.is_free());
        assert_eq!(FatEntry::Data(42).next_cluster(), Some(42));
        assert_eq!(FatEntry::EndOfChain.next_cluster(), None);
    }

    #[test]
    fn test_cluster_chain() {
        // Build a simple FAT: cluster 2 -> 3 -> 4 -> EOC
        let mut fat_data = vec![0u8; 32];
        // Entry 0 and 1 are reserved
        // Entry 2: points to cluster 3
        fat_data[8..12].copy_from_slice(&3u32.to_le_bytes());
        // Entry 3: points to cluster 4
        fat_data[12..16].copy_from_slice(&4u32.to_le_bytes());
        // Entry 4: end of chain
        fat_data[16..20].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        let fat = FatTable::new(&fat_data);
        let chain = fat.get_cluster_chain(2);

        assert_eq!(chain, vec![2, 3, 4]);
    }
}
