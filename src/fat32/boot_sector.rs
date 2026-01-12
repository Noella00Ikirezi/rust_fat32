//! Boot Sector FAT32 - Parse les 512 premiers octets du filesystem

/// Structure du boot sector contenant les paramètres FAT32
#[derive(Debug, Clone)]
pub struct BootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub sectors_per_fat: u32,
    pub root_cluster: u32,
    pub total_sectors: u32,
}

impl BootSector {
    /// Parse le boot sector depuis 512 octets bruts
    pub fn from_bytes(data: &[u8; 512]) -> Option<Self> {
        if data[510] != 0x55 || data[511] != 0xAA {
            return None;
        }

        Some(BootSector {
            bytes_per_sector: u16::from_le_bytes([data[11], data[12]]),
            sectors_per_cluster: data[13],
            reserved_sectors: u16::from_le_bytes([data[14], data[15]]),
            fat_count: data[16],
            sectors_per_fat: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            root_cluster: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
            total_sectors: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
        })
    }

    /// Retourne le secteur de début de la table FAT
    #[inline]
    pub fn fat_start_sector(&self) -> u32 {
        self.reserved_sectors as u32
    }

    /// Retourne le secteur de début de la région de données
    #[inline]
    pub fn data_start_sector(&self) -> u32 {
        self.reserved_sectors as u32 + (self.fat_count as u32 * self.sectors_per_fat)
    }

    /// Convertit un numéro de cluster en numéro de secteur
    #[inline]
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector() + (cluster - 2) * self.sectors_per_cluster as u32
    }

    /// Retourne le nombre d'octets par cluster
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
        data[510] = 0x00;
        data[511] = 0x00;
        assert!(BootSector::from_bytes(&data).is_none());
    }

    #[test]
    fn test_valid_boot_sector() {
        let mut data = [0u8; 512];
        data[510] = 0x55;
        data[511] = 0xAA;
        data[11] = 0x00;
        data[12] = 0x02;
        data[13] = 8;
        data[14] = 32;
        data[15] = 0;
        data[16] = 2;
        data[44] = 2;

        let bs = BootSector::from_bytes(&data).unwrap();
        assert_eq!(bs.bytes_per_sector, 512);
        assert_eq!(bs.sectors_per_cluster, 8);
        assert_eq!(bs.reserved_sectors, 32);
        assert_eq!(bs.fat_count, 2);
        assert_eq!(bs.root_cluster, 2);
    }
}
