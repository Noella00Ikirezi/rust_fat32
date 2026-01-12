//! Gestion des entrées de répertoire FAT32 (32 octets par entrée)

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// Flags d'attributs des entrées
pub const ATTR_READ_ONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME_ID: u8 = 0x08;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;
pub const ATTR_LONG_NAME: u8 = 0x0F;

/// Entrée de répertoire FAT32 (32 octets)
#[derive(Clone, Debug)]
pub struct DirEntry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attr: u8,
    pub cluster_high: u16,
    pub cluster_low: u16,
    pub size: u32,
    pub create_time: u16,
    pub create_date: u16,
    pub access_date: u16,
    pub modify_time: u16,
    pub modify_date: u16,
}

impl DirEntry {
    /// Parse une entrée de répertoire depuis 32 octets
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        let first_byte = data[0];
        if first_byte == 0x00 || first_byte == 0xE5 {
            return None;
        }

        let mut name = [0u8; 8];
        let mut ext = [0u8; 3];
        name.copy_from_slice(&data[0..8]);
        ext.copy_from_slice(&data[8..11]);

        Some(DirEntry {
            name,
            ext,
            attr: data[11],
            create_time: u16::from_le_bytes([data[14], data[15]]),
            create_date: u16::from_le_bytes([data[16], data[17]]),
            access_date: u16::from_le_bytes([data[18], data[19]]),
            cluster_high: u16::from_le_bytes([data[20], data[21]]),
            modify_time: u16::from_le_bytes([data[22], data[23]]),
            modify_date: u16::from_le_bytes([data[24], data[25]]),
            cluster_low: u16::from_le_bytes([data[26], data[27]]),
            size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        })
    }

    /// Retourne le numéro de cluster complet (32-bit)
    #[inline]
    pub fn cluster(&self) -> u32 {
        ((self.cluster_high as u32) << 16) | (self.cluster_low as u32)
    }

    /// Vérifie si c'est un répertoire
    #[inline]
    pub fn is_directory(&self) -> bool {
        self.attr & ATTR_DIRECTORY != 0
    }

    /// Vérifie si c'est caché
    #[inline]
    pub fn is_hidden(&self) -> bool {
        self.attr & ATTR_HIDDEN != 0
    }

    /// Vérifie si c'est le label du volume
    #[inline]
    pub fn is_volume_label(&self) -> bool {
        self.attr & ATTR_VOLUME_ID != 0
    }

    /// Vérifie si c'est une entrée LFN
    #[inline]
    pub fn is_long_name(&self) -> bool {
        self.attr == ATTR_LONG_NAME
    }

    /// Vérifie si c'est en lecture seule
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.attr & ATTR_READ_ONLY != 0
    }

    /// Vérifie si c'est un fichier système
    #[inline]
    pub fn is_system(&self) -> bool {
        self.attr & ATTR_SYSTEM != 0
    }

    /// Vérifie si c'est l'entrée "."
    pub fn is_dot(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b' '
    }

    /// Vérifie si c'est l'entrée ".."
    pub fn is_dotdot(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b'.' && self.name[2] == b' '
    }

    /// Retourne le nom d'affichage (NAME.EXT)
    pub fn display_name(&self) -> String {
        if self.is_dot() {
            return String::from(".");
        }
        if self.is_dotdot() {
            return String::from("..");
        }

        let name_part: String = self.name.iter()
            .take_while(|&&b| b != 0x20 && b != 0x00)
            .map(|&b| b as char)
            .collect();

        let ext_part: String = self.ext.iter()
            .take_while(|&&b| b != 0x20 && b != 0x00)
            .map(|&b| b as char)
            .collect();

        if ext_part.is_empty() {
            name_part
        } else {
            alloc::format!("{}.{}", name_part, ext_part)
        }
    }

    /// Retourne le nom court brut (format 8.3)
    pub fn short_name(&self) -> String {
        let mut result = String::new();
        for &b in &self.name {
            result.push(b as char);
        }
        result.push('.');
        for &b in &self.ext {
            result.push(b as char);
        }
        result
    }
}

/// Entrée de nom long (LFN)
#[derive(Clone, Debug)]
pub struct LfnEntry {
    pub sequence: u8,
    pub name1: [u16; 5],
    pub name2: [u16; 6],
    pub name3: [u16; 2],
    pub checksum: u8,
}

impl LfnEntry {
    /// Parse une entrée LFN depuis 32 octets
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 || data[11] != ATTR_LONG_NAME {
            return None;
        }

        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];

        for i in 0..5 {
            let offset = 1 + i * 2;
            name1[i] = u16::from_le_bytes([data[offset], data[offset + 1]]);
        }

        for i in 0..6 {
            let offset = 14 + i * 2;
            name2[i] = u16::from_le_bytes([data[offset], data[offset + 1]]);
        }

        for i in 0..2 {
            let offset = 28 + i * 2;
            name3[i] = u16::from_le_bytes([data[offset], data[offset + 1]]);
        }

        Some(LfnEntry {
            sequence: data[0],
            name1,
            name2,
            name3,
            checksum: data[13],
        })
    }

    /// Vérifie si c'est la dernière entrée LFN
    pub fn is_last(&self) -> bool {
        self.sequence & 0x40 != 0
    }

    /// Retourne le numéro de séquence (1-20)
    pub fn order(&self) -> u8 {
        self.sequence & 0x1F
    }

    /// Extrait les caractères de cette entrée LFN
    pub fn get_chars(&self) -> Vec<char> {
        let mut chars = Vec::new();

        for &c in &self.name1 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }

        for &c in &self.name2 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }

        for &c in &self.name3 {
            if c == 0x0000 || c == 0xFFFF {
                return chars;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }

        chars
    }
}

/// Parse toutes les entrées d'un répertoire
pub fn parse_directory(data: &[u8]) -> Vec<DirEntry> {
    let mut entries = Vec::new();

    for chunk in data.chunks(32) {
        if chunk.len() < 32 || chunk[0] == 0x00 {
            break;
        }

        if let Some(entry) = DirEntry::from_bytes(chunk) {
            if !entry.is_long_name() && !entry.is_volume_label() {
                entries.push(entry);
            }
        }
    }

    entries
}

/// Parse le répertoire avec support des noms longs
pub fn parse_directory_with_lfn(data: &[u8]) -> Vec<(DirEntry, Option<String>)> {
    let mut entries = Vec::new();
    let mut lfn_parts: Vec<(u8, Vec<char>)> = Vec::new();

    for chunk in data.chunks(32) {
        if chunk.len() < 32 || chunk[0] == 0x00 {
            break;
        }

        if chunk[11] == ATTR_LONG_NAME && chunk[0] != 0xE5 {
            if let Some(lfn) = LfnEntry::from_bytes(chunk) {
                lfn_parts.push((lfn.order(), lfn.get_chars()));
            }
            continue;
        }

        if let Some(entry) = DirEntry::from_bytes(chunk) {
            if entry.is_volume_label() {
                lfn_parts.clear();
                continue;
            }

            let long_name = if !lfn_parts.is_empty() {
                lfn_parts.sort_by_key(|(order, _)| *order);
                let name: String = lfn_parts.iter()
                    .flat_map(|(_, chars)| chars.iter())
                    .collect();
                lfn_parts.clear();
                Some(name)
            } else {
                None
            };

            entries.push((entry, long_name));
        } else {
            lfn_parts.clear();
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_entry_parsing() {
        let mut data = [0u8; 32];
        data[0..8].copy_from_slice(b"TEST    ");
        data[8..11].copy_from_slice(b"TXT");
        data[11] = ATTR_ARCHIVE;

        let entry = DirEntry::from_bytes(&data).unwrap();
        assert_eq!(entry.display_name(), "TEST.TXT");
        assert!(!entry.is_directory());
    }

    #[test]
    fn test_directory_entry() {
        let mut data = [0u8; 32];
        data[0..8].copy_from_slice(b"DOCS    ");
        data[8..11].copy_from_slice(b"   ");
        data[11] = ATTR_DIRECTORY;

        let entry = DirEntry::from_bytes(&data).unwrap();
        assert_eq!(entry.display_name(), "DOCS");
        assert!(entry.is_directory());
    }

    #[test]
    fn test_cluster_calculation() {
        let mut data = [0u8; 32];
        data[0..8].copy_from_slice(b"FILE    ");
        data[11] = ATTR_ARCHIVE;
        data[20] = 0x01;
        data[21] = 0x00;
        data[26] = 0x00;
        data[27] = 0x02;

        let entry = DirEntry::from_bytes(&data).unwrap();
        assert_eq!(entry.cluster(), 0x00010200);
    }

    #[test]
    fn test_dot_entries() {
        let mut data = [0u8; 32];
        data[0..8].copy_from_slice(b".       ");
        data[11] = ATTR_DIRECTORY;
        let entry = DirEntry::from_bytes(&data).unwrap();
        assert!(entry.is_dot());
        assert_eq!(entry.display_name(), ".");

        data[0..8].copy_from_slice(b"..      ");
        let entry = DirEntry::from_bytes(&data).unwrap();
        assert!(entry.is_dotdot());
        assert_eq!(entry.display_name(), "..");
    }

    #[test]
    fn test_deleted_entry() {
        let mut data = [0u8; 32];
        data[0] = 0xE5;
        assert!(DirEntry::from_bytes(&data).is_none());
    }

    #[test]
    fn test_end_marker() {
        let data = [0u8; 32];
        assert!(DirEntry::from_bytes(&data).is_none());
    }
}
