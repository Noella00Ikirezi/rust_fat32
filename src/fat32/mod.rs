//! Implémentation du système de fichiers FAT32 (lecture seule, compatible no_std)

pub mod boot_sector;
pub mod fat;
pub mod directory;

pub use boot_sector::BootSector;
pub use fat::{FatTable, FatEntry};
pub use directory::{DirEntry, parse_directory, parse_directory_with_lfn};
pub use directory::{ATTR_READ_ONLY, ATTR_HIDDEN, ATTR_SYSTEM, ATTR_VOLUME_ID,
                   ATTR_DIRECTORY, ATTR_ARCHIVE, ATTR_LONG_NAME};

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

/// Interface du système de fichiers FAT32
pub struct Fat32<'a> {
    disk_data: &'a [u8],
    boot_sector: BootSector,
}

impl<'a> Fat32<'a> {
    /// Crée un nouveau filesystem FAT32 depuis les données brutes du disque
    pub fn new(disk_data: &'a [u8]) -> Option<Self> {
        if disk_data.len() < 512 {
            return None;
        }

        let boot_bytes: [u8; 512] = disk_data[0..512].try_into().ok()?;
        let boot_sector = BootSector::from_bytes(&boot_bytes)?;

        if boot_sector.bytes_per_sector == 0 || boot_sector.sectors_per_cluster == 0 {
            return None;
        }

        Some(Fat32 { disk_data, boot_sector })
    }

    /// Retourne les informations du boot sector
    #[inline]
    pub fn boot_sector(&self) -> &BootSector {
        &self.boot_sector
    }

    /// Retourne le numéro de cluster du répertoire racine
    #[inline]
    pub fn root_cluster(&self) -> u32 {
        self.boot_sector.root_cluster
    }

    /// Retourne le nombre d'octets par secteur
    #[inline]
    pub fn bytes_per_sector(&self) -> u16 {
        self.boot_sector.bytes_per_sector
    }

    /// Retourne le nombre d'octets par cluster
    #[inline]
    pub fn bytes_per_cluster(&self) -> u32 {
        self.boot_sector.bytes_per_cluster()
    }

    /// Retourne le lecteur de table FAT
    fn fat_table(&self) -> FatTable<'_> {
        let start = self.boot_sector.fat_start_sector() as usize
            * self.boot_sector.bytes_per_sector as usize;
        let size = self.boot_sector.sectors_per_fat as usize
            * self.boot_sector.bytes_per_sector as usize;

        let end = (start + size).min(self.disk_data.len());
        FatTable::new(&self.disk_data[start..end])
    }

    /// Lit un seul cluster
    fn read_cluster(&self, cluster: u32) -> &[u8] {
        if cluster < 2 {
            return &[];
        }

        let sector = self.boot_sector.cluster_to_sector(cluster);
        let bytes_per_cluster = self.boot_sector.bytes_per_cluster() as usize;
        let start = sector as usize * self.boot_sector.bytes_per_sector as usize;
        let end = start + bytes_per_cluster;

        if end > self.disk_data.len() {
            return &[];
        }

        &self.disk_data[start..end]
    }

    /// Lit une chaîne complète de clusters
    pub fn read_cluster_chain(&self, start: u32) -> Vec<u8> {
        let fat = self.fat_table();
        let chain = fat.get_cluster_chain(start);
        let mut data = Vec::new();

        for cluster in chain {
            data.extend_from_slice(self.read_cluster(cluster));
        }

        data
    }

    /// Lit les entrées d'un répertoire
    pub fn read_directory(&self, cluster: u32) -> Vec<DirEntry> {
        let data = self.read_cluster_chain(cluster);
        parse_directory(&data)
    }

    /// Lit les entrées d'un répertoire avec support des noms longs
    pub fn read_directory_with_lfn(&self, cluster: u32) -> Vec<(DirEntry, Option<String>)> {
        let data = self.read_cluster_chain(cluster);
        parse_directory_with_lfn(&data)
    }

    /// Cherche une entrée par nom dans un répertoire (insensible à la casse)
    pub fn find_entry(&self, dir_cluster: u32, name: &str) -> Option<DirEntry> {
        let entries = self.read_directory_with_lfn(dir_cluster);
        let name_upper = name.to_ascii_uppercase();

        for (entry, long_name) in entries {
            if let Some(ref ln) = long_name {
                if ln.to_ascii_uppercase() == name_upper {
                    return Some(entry);
                }
            }

            if entry.display_name().to_ascii_uppercase() == name_upper {
                return Some(entry);
            }
        }

        None
    }

    /// Lit le contenu d'un fichier
    pub fn read_file(&self, entry: &DirEntry) -> Vec<u8> {
        if entry.is_directory() {
            return Vec::new();
        }

        let mut data = self.read_cluster_chain(entry.cluster());
        let actual_size = entry.size as usize;

        if data.len() > actual_size {
            data.truncate(actual_size);
        }

        data
    }

    /// Résout un chemin et retourne l'entrée correspondante
    pub fn resolve_path(&self, path: &str, current_cluster: u32) -> Option<DirEntry> {
        let path = path.trim();

        if path.is_empty() || path == "/" {
            return None;
        }

        let (start_cluster, path_str) = if path.starts_with('/') {
            (self.root_cluster(), &path[1..])
        } else {
            (current_cluster, path)
        };

        let components: Vec<&str> = path_str
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if components.is_empty() {
            return None;
        }

        let mut cluster = start_cluster;

        for component in &components[..components.len() - 1] {
            match self.find_entry(cluster, component) {
                Some(entry) if entry.is_directory() => {
                    cluster = entry.cluster();
                }
                _ => return None,
            }
        }

        let final_name = components.last()?;
        self.find_entry(cluster, final_name)
    }

    /// Retourne la taille totale du filesystem en octets
    pub fn total_size(&self) -> u64 {
        self.boot_sector.total_sectors as u64 * self.boot_sector.bytes_per_sector as u64
    }

    /// Calcule l'espace libre (opération coûteuse)
    pub fn free_space(&self) -> u64 {
        let fat = self.fat_table();
        let data_clusters = (self.boot_sector.total_sectors
            - self.boot_sector.data_start_sector())
            / self.boot_sector.sectors_per_cluster as u32;

        let free_clusters = fat.count_free_clusters(data_clusters);
        free_clusters as u64 * self.boot_sector.bytes_per_cluster() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_fat32_image() -> Vec<u8> {
        let mut data = vec![0u8; 1024 * 1024];

        data[11] = 0x00;
        data[12] = 0x02;
        data[13] = 1;
        data[14] = 32;
        data[15] = 0;
        data[16] = 2;
        let total_sectors: u32 = 2048;
        data[32..36].copy_from_slice(&total_sectors.to_le_bytes());
        data[36..40].copy_from_slice(&16u32.to_le_bytes());
        data[44..48].copy_from_slice(&2u32.to_le_bytes());
        data[510] = 0x55;
        data[511] = 0xAA;

        let fat_start = 32 * 512;
        data[fat_start + 8..fat_start + 12].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

        let root_dir = 64 * 512;
        data[root_dir..root_dir + 8].copy_from_slice(b"TEST    ");
        data[root_dir + 8..root_dir + 11].copy_from_slice(b"TXT");
        data[root_dir + 11] = ATTR_ARCHIVE;
        data[root_dir + 28..root_dir + 32].copy_from_slice(&100u32.to_le_bytes());

        data
    }

    #[test]
    fn test_fat32_creation() {
        let image = create_minimal_fat32_image();
        let fs = Fat32::new(&image);
        assert!(fs.is_some());

        let fs = fs.unwrap();
        assert_eq!(fs.root_cluster(), 2);
        assert_eq!(fs.bytes_per_sector(), 512);
    }

    #[test]
    fn test_read_root_directory() {
        let image = create_minimal_fat32_image();
        let fs = Fat32::new(&image).unwrap();

        let entries = fs.read_directory(fs.root_cluster());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name(), "TEST.TXT");
    }

    #[test]
    fn test_find_entry() {
        let image = create_minimal_fat32_image();
        let fs = Fat32::new(&image).unwrap();

        let entry = fs.find_entry(fs.root_cluster(), "test.txt");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().display_name(), "TEST.TXT");

        let entry = fs.find_entry(fs.root_cluster(), "notfound.txt");
        assert!(entry.is_none());
    }

    #[test]
    fn test_invalid_image() {
        let data = vec![0u8; 512];
        assert!(Fat32::new(&data).is_none());

        let data = vec![0u8; 100];
        assert!(Fat32::new(&data).is_none());
    }
}
