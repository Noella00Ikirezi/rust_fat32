//! Table FAT (File Allocation Table) - Gère les chaînes de clusters

extern crate alloc;
use alloc::vec::Vec;

/// Types d'entrées FAT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatEntry {
    Free,
    Reserved,
    Data(u32),
    BadCluster,
    EndOfChain,
}

impl FatEntry {
    /// Parse une entrée FAT 32-bit brute
    pub fn from_raw(value: u32) -> Self {
        match value & 0x0FFFFFFF {
            0x00000000 => FatEntry::Free,
            0x00000001 => FatEntry::Reserved,
            0x0FFFFFF7 => FatEntry::BadCluster,
            0x0FFFFFF8..=0x0FFFFFFF => FatEntry::EndOfChain,
            n => FatEntry::Data(n),
        }
    }

    /// Vérifie si c'est la fin de chaîne
    #[inline]
    pub fn is_end(&self) -> bool {
        matches!(self, FatEntry::EndOfChain)
    }

    /// Vérifie si le cluster est libre
    #[inline]
    pub fn is_free(&self) -> bool {
        matches!(self, FatEntry::Free)
    }

    /// Retourne le prochain cluster si c'est une entrée de données
    #[inline]
    pub fn next_cluster(&self) -> Option<u32> {
        match self {
            FatEntry::Data(n) => Some(*n),
            _ => None,
        }
    }
}

/// Lecteur de table FAT
pub struct FatTable<'a> {
    data: &'a [u8],
}

impl<'a> FatTable<'a> {
    /// Crée un nouveau lecteur de table FAT
    pub fn new(data: &'a [u8]) -> Self {
        FatTable { data }
    }

    /// Récupère l'entrée FAT pour un cluster
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

    /// Récupère la chaîne complète de clusters
    pub fn get_cluster_chain(&self, start: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut current = start;
        const MAX_CHAIN_LENGTH: usize = 1_000_000;

        loop {
            if current < 2 {
                break;
            }
            if chain.len() >= MAX_CHAIN_LENGTH {
                break;
            }

            chain.push(current);

            match self.get_entry(current) {
                FatEntry::Data(next) => {
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

    /// Compte les clusters libres dans la FAT
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
        let mut fat_data = vec![0u8; 32];
        fat_data[8..12].copy_from_slice(&3u32.to_le_bytes());
        fat_data[12..16].copy_from_slice(&4u32.to_le_bytes());
        fat_data[16..20].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        let fat = FatTable::new(&fat_data);
        let chain = fat.get_cluster_chain(2);

        assert_eq!(chain, vec![2, 3, 4]);
    }
}
