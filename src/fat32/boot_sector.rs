//! FAT32 Boot Sector Parser
//!
//! Parses the first 512 bytes of a FAT32 filesystem to extract
//! critical filesystem parameters.

/// Boot sector structure containing FAT32 filesystem parameters
#[derive(Debug, Clone)]
pub struct BootSector {
    /// Bytes per sector (usually 512)
    pub bytes_per_sector: u16,
    /// Sectors per cluster (power of 2)
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors before first FAT
    pub reserved_sectors: u16,
    /// Number of FAT tables (usually 2)
    pub fat_count: u8,
    /// Sectors per FAT table
    pub sectors_per_fat: u32,
    /// Root directory starting cluster
    pub root_cluster: u32,
    /// Total sectors in filesystem
    pub total_sectors: u32,
}

impl BootSector {
    /// Parse boot sector from raw bytes
    ///
    /// # Arguments
    /// * `data` - Exactly 512 bytes of boot sector data
    ///
    /// # Returns
    /// * `Some(BootSector)` if valid FAT32 boot sector
    /// * `None` if signature invalid or parsing fails
    pub fn from_bytes(data: &[u8; 512]) -> Option<Self> {
        // Verify boot sector signature (0x55AA at offset 510-511)
        if data[510] != 0x55 || data[511] != 0xAA {
            return None;
        }

        Some(BootSector {
            // Offset 11-12: Bytes per sector
            bytes_per_sector: u16::from_le_bytes([data[11], data[12]]),
            // Offset 13: Sectors per cluster
            sectors_per_cluster: data[13],
            // Offset 14-15: Reserved sector count
            reserved_sectors: u16::from_le_bytes([data[14], data[15]]),
            // Offset 16: Number of FATs
            fat_count: data[16],
            // Offset 36-39: FAT32 sectors per FAT
            sectors_per_fat: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            // Offset 44-47: Root cluster number
            root_cluster: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
            // Offset 32-35: Total sectors (32-bit)
            total_sectors: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
        })
    }

    /// Calculate the starting sector of the FAT table
    #[inline]
    pub fn fat_start_sector(&self) -> u32 {
        self.reserved_sectors as u32
    }

    /// Calculate the starting sector of the data region
    #[inline]
    pub fn data_start_sector(&self) -> u32 {
        self.reserved_sectors as u32 + (self.fat_count as u32 * self.sectors_per_fat)
    }

    /// Convert cluster number to sector number
    ///
    /// # Arguments
    /// * `cluster` - Cluster number (must be >= 2)
    ///
    /// # Returns
    /// First sector of the cluster
    #[inline]
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector() + (cluster - 2) * self.sectors_per_cluster as u32
    }

    /// Calculate bytes per cluster
    #[inline]
    pub fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector as u32 * self.sectors_per_cluster as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_signature() {
        let mut data = [0u8; 512];
        // Wrong signature
        data[510] = 0x00;
        data[511] = 0x00;
        assert!(BootSector::from_bytes(&data).is_none());
    }

    #[test]
    fn test_valid_boot_sector() {
        let mut data = [0u8; 512];
        // Valid signature
        data[510] = 0x55;
        data[511] = 0xAA;
        // Bytes per sector = 512
        data[11] = 0x00;
        data[12] = 0x02;
        // Sectors per cluster = 8
        data[13] = 8;
        // Reserved sectors = 32
        data[14] = 32;
        data[15] = 0;
        // FAT count = 2
        data[16] = 2;
        // Root cluster = 2
        data[44] = 2;

        let bs = BootSector::from_bytes(&data).unwrap();
        assert_eq!(bs.bytes_per_sector, 512);
        assert_eq!(bs.sectors_per_cluster, 8);
        assert_eq!(bs.reserved_sectors, 32);
        assert_eq!(bs.fat_count, 2);
        assert_eq!(bs.root_cluster, 2);
    }
}
