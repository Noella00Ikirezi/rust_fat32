//! FAT32 Directory Entry handling
//!
//! Directory entries are 32-byte structures containing file metadata.
//! Supports both short (8.3) and long filename entries.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// Directory entry attribute flags
/// Read-only file
pub const ATTR_READ_ONLY: u8 = 0x01;
/// Hidden file
pub const ATTR_HIDDEN: u8 = 0x02;
/// System file
pub const ATTR_SYSTEM: u8 = 0x04;
/// Volume label (root directory only)
pub const ATTR_VOLUME_ID: u8 = 0x08;
/// Directory
pub const ATTR_DIRECTORY: u8 = 0x10;
/// Archive flag
pub const ATTR_ARCHIVE: u8 = 0x20;
/// Long filename entry (combination of other flags)
pub const ATTR_LONG_NAME: u8 = 0x0F;

/// FAT32 Directory Entry (32 bytes)
#[derive(Clone, Debug)]
pub struct DirEntry {
    /// Short filename (8 chars, space-padded)
    pub name: [u8; 8],
    /// Extension (3 chars, space-padded)
    pub ext: [u8; 3],
    /// File attributes
    pub attr: u8,
    /// High 16 bits of cluster number
    pub cluster_high: u16,
    /// Low 16 bits of cluster number
    pub cluster_low: u16,
    /// File size in bytes
    pub size: u32,
    /// Creation time (raw)
    pub create_time: u16,
    /// Creation date (raw)
    pub create_date: u16,
    /// Last access date (raw)
    pub access_date: u16,
    /// Last modification time (raw)
    pub modify_time: u16,
    /// Last modification date (raw)
    pub modify_date: u16,
}

impl DirEntry {
    /// Parse directory entry from 32 bytes
    ///
    /// # Arguments
    /// * `data` - At least 32 bytes of directory entry data
    ///
    /// # Returns
    /// * `Some(DirEntry)` if valid entry
    /// * `None` if entry is deleted (0xE5) or end marker (0x00)
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        let first_byte = data[0];

        // 0x00 = end of directory entries
        if first_byte == 0x00 {
            return None;
        }

        // 0xE5 = deleted entry
        if first_byte == 0xE5 {
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

    /// Get full 32-bit cluster number
    #[inline]
    pub fn cluster(&self) -> u32 {
        ((self.cluster_high as u32) << 16) | (self.cluster_low as u32)
    }

    /// Check if entry is a directory
    #[inline]
    pub fn is_directory(&self) -> bool {
        self.attr & ATTR_DIRECTORY != 0
    }

    /// Check if entry is hidden
    #[inline]
    pub fn is_hidden(&self) -> bool {
        self.attr & ATTR_HIDDEN != 0
    }

    /// Check if entry is the volume label
    #[inline]
    pub fn is_volume_label(&self) -> bool {
        self.attr & ATTR_VOLUME_ID != 0
    }

    /// Check if entry is a long filename entry
    #[inline]
    pub fn is_long_name(&self) -> bool {
        self.attr == ATTR_LONG_NAME
    }

    /// Check if entry is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.attr & ATTR_READ_ONLY != 0
    }

    /// Check if entry is a system file
    #[inline]
    pub fn is_system(&self) -> bool {
        self.attr & ATTR_SYSTEM != 0
    }

    /// Check if this is the "." entry
    pub fn is_dot(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b' '
    }

    /// Check if this is the ".." entry
    pub fn is_dotdot(&self) -> bool {
        self.name[0] == b'.' && self.name[1] == b'.' && self.name[2] == b' '
    }

    /// Get display name in standard format (NAME.EXT)
    ///
    /// Removes trailing spaces and combines name with extension.
    pub fn display_name(&self) -> String {
        // Handle special entries
        if self.is_dot() {
            return String::from(".");
        }
        if self.is_dotdot() {
            return String::from("..");
        }

        // Extract name part (remove trailing spaces)
        let name_part: String = self.name.iter()
            .take_while(|&&b| b != 0x20 && b != 0x00)
            .map(|&b| b as char)
            .collect();

        // Extract extension part (remove trailing spaces)
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

    /// Get short name as stored (8.3 format with spaces)
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

/// Long Filename Entry (LFN)
///
/// FAT32 supports long filenames using special directory entries
/// that precede the standard 8.3 entry.
#[derive(Clone, Debug)]
pub struct LfnEntry {
    /// Sequence number (1-20, 0x40 flag for last entry)
    pub sequence: u8,
    /// Characters 1-5 of this segment (UCS-2)
    pub name1: [u16; 5],
    /// Characters 6-11 of this segment (UCS-2)
    pub name2: [u16; 6],
    /// Characters 12-13 of this segment (UCS-2)
    pub name3: [u16; 2],
    /// Checksum of short filename
    pub checksum: u8,
}

impl LfnEntry {
    /// Parse LFN entry from 32 bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        // Verify this is an LFN entry
        if data[11] != ATTR_LONG_NAME {
            return None;
        }

        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];

        // Parse UCS-2 characters
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

    /// Check if this is the last LFN entry (has 0x40 flag)
    pub fn is_last(&self) -> bool {
        self.sequence & 0x40 != 0
    }

    /// Get sequence number (1-20)
    pub fn order(&self) -> u8 {
        self.sequence & 0x1F
    }

    /// Extract characters from this LFN entry
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

/// Parse all directory entries from raw directory data
///
/// Handles both short (8.3) and long filename entries.
///
/// # Arguments
/// * `data` - Raw bytes of directory cluster(s)
///
/// # Returns
/// Vector of valid directory entries (excluding LFN entries)
pub fn parse_directory(data: &[u8]) -> Vec<DirEntry> {
    let mut entries = Vec::new();

    for chunk in data.chunks(32) {
        if chunk.len() < 32 {
            break;
        }

        // 0x00 marks end of directory
        if chunk[0] == 0x00 {
            break;
        }

        if let Some(entry) = DirEntry::from_bytes(chunk) {
            // Skip LFN entries and volume labels
            if !entry.is_long_name() && !entry.is_volume_label() {
                entries.push(entry);
            }
        }
    }

    entries
}

/// Parse directory with long filename support
///
/// Returns entries with their full long filenames if available.
///
/// # Arguments
/// * `data` - Raw bytes of directory cluster(s)
///
/// # Returns
/// Vector of (DirEntry, Option<String>) where String is the long filename
pub fn parse_directory_with_lfn(data: &[u8]) -> Vec<(DirEntry, Option<String>)> {
    let mut entries = Vec::new();
    let mut lfn_parts: Vec<(u8, Vec<char>)> = Vec::new();

    for chunk in data.chunks(32) {
        if chunk.len() < 32 {
            break;
        }

        if chunk[0] == 0x00 {
            break;
        }

        // Check if this is an LFN entry
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

            // Reconstruct long filename if we have LFN entries
            let long_name = if !lfn_parts.is_empty() {
                // Sort by sequence number and concatenate
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
        data[20] = 0x01; // cluster_high low byte
        data[21] = 0x00; // cluster_high high byte
        data[26] = 0x00; // cluster_low low byte
        data[27] = 0x02; // cluster_low high byte

        let entry = DirEntry::from_bytes(&data).unwrap();
        // cluster = (0x0001 << 16) | 0x0200 = 0x00010200
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
        data[0] = 0xE5; // Deleted marker
        assert!(DirEntry::from_bytes(&data).is_none());
    }

    #[test]
    fn test_end_marker() {
        let data = [0u8; 32]; // First byte is 0x00
        assert!(DirEntry::from_bytes(&data).is_none());
    }
}
