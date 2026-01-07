//! FAT32 Integration Tests
//!
//! These tests verify the FAT32 filesystem implementation.

use fat32_exam::fat32::*;

/// Create a minimal FAT32 image for testing
fn create_test_image() -> Vec<u8> {
    let mut data = vec![0u8; 1024 * 1024]; // 1MB image

    // === Boot Sector (Sector 0) ===
    // Jump instruction
    data[0] = 0xEB;
    data[1] = 0x58;
    data[2] = 0x90;

    // OEM Name
    data[3..11].copy_from_slice(b"MSWIN4.1");

    // Bytes per sector = 512 (0x0200)
    data[11] = 0x00;
    data[12] = 0x02;

    // Sectors per cluster = 1
    data[13] = 1;

    // Reserved sectors = 32
    data[14] = 32;
    data[15] = 0;

    // Number of FATs = 2
    data[16] = 2;

    // Root entry count (FAT32 = 0)
    data[17] = 0;
    data[18] = 0;

    // Total sectors 16-bit (FAT32 = 0)
    data[19] = 0;
    data[20] = 0;

    // Media type
    data[21] = 0xF8;

    // FAT size 16-bit (FAT32 = 0)
    data[22] = 0;
    data[23] = 0;

    // Sectors per track
    data[24] = 63;
    data[25] = 0;

    // Number of heads
    data[26] = 255;
    data[27] = 0;

    // Hidden sectors
    data[28..32].copy_from_slice(&0u32.to_le_bytes());

    // Total sectors 32-bit = 2048
    let total_sectors: u32 = 2048;
    data[32..36].copy_from_slice(&total_sectors.to_le_bytes());

    // === FAT32 Extended Boot Sector ===

    // Sectors per FAT (FAT32) = 16
    data[36..40].copy_from_slice(&16u32.to_le_bytes());

    // Extended flags
    data[40] = 0;
    data[41] = 0;

    // FS Version
    data[42] = 0;
    data[43] = 0;

    // Root cluster = 2
    data[44..48].copy_from_slice(&2u32.to_le_bytes());

    // FSInfo sector
    data[48] = 1;
    data[49] = 0;

    // Backup boot sector
    data[50] = 6;
    data[51] = 0;

    // Boot signature
    data[510] = 0x55;
    data[511] = 0xAA;

    // === FAT Table (starts at sector 32, offset 16384) ===
    let fat_start = 32 * 512;

    // FAT entry 0: Media type
    data[fat_start..fat_start + 4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());

    // FAT entry 1: End of chain marker
    data[fat_start + 4..fat_start + 8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // FAT entry 2 (root directory): End of chain
    data[fat_start + 8..fat_start + 12].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // FAT entry 3 (DOCS directory): End of chain
    data[fat_start + 12..fat_start + 16].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // FAT entry 4 (test file): Points to cluster 5
    data[fat_start + 16..fat_start + 20].copy_from_slice(&5u32.to_le_bytes());

    // FAT entry 5: End of chain
    data[fat_start + 20..fat_start + 24].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // === Data Region (starts at sector 64, offset 32768) ===
    // Cluster 2 = Root directory
    let root_dir = 64 * 512;

    // Entry 1: TEST.TXT file
    data[root_dir..root_dir + 8].copy_from_slice(b"TEST    ");
    data[root_dir + 8..root_dir + 11].copy_from_slice(b"TXT");
    data[root_dir + 11] = 0x20; // Archive
    data[root_dir + 26..root_dir + 28].copy_from_slice(&4u16.to_le_bytes()); // Cluster low = 4
    data[root_dir + 28..root_dir + 32].copy_from_slice(&13u32.to_le_bytes()); // Size = 13 bytes

    // Entry 2: DOCS directory
    data[root_dir + 32..root_dir + 40].copy_from_slice(b"DOCS    ");
    data[root_dir + 40..root_dir + 43].copy_from_slice(b"   ");
    data[root_dir + 43] = 0x10; // Directory
    data[root_dir + 58..root_dir + 60].copy_from_slice(&3u16.to_le_bytes()); // Cluster low = 3

    // Entry 3: README.MD file
    data[root_dir + 64..root_dir + 72].copy_from_slice(b"README  ");
    data[root_dir + 72..root_dir + 75].copy_from_slice(b"MD ");
    data[root_dir + 75] = 0x20; // Archive
    data[root_dir + 90..root_dir + 92].copy_from_slice(&6u16.to_le_bytes()); // Cluster low = 6
    data[root_dir + 92..root_dir + 96].copy_from_slice(&19u32.to_le_bytes()); // Size = 19 bytes

    // FAT entry 6: End of chain
    data[fat_start + 24..fat_start + 28].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // === Cluster 3 = DOCS directory ===
    let docs_dir = 65 * 512;

    // . entry
    data[docs_dir..docs_dir + 8].copy_from_slice(b".       ");
    data[docs_dir + 8..docs_dir + 11].copy_from_slice(b"   ");
    data[docs_dir + 11] = 0x10;
    data[docs_dir + 26..docs_dir + 28].copy_from_slice(&3u16.to_le_bytes());

    // .. entry
    data[docs_dir + 32..docs_dir + 40].copy_from_slice(b"..      ");
    data[docs_dir + 40..docs_dir + 43].copy_from_slice(b"   ");
    data[docs_dir + 43] = 0x10;
    data[docs_dir + 58..docs_dir + 60].copy_from_slice(&0u16.to_le_bytes()); // 0 = root

    // INFO.TXT in DOCS
    data[docs_dir + 64..docs_dir + 72].copy_from_slice(b"INFO    ");
    data[docs_dir + 72..docs_dir + 75].copy_from_slice(b"TXT");
    data[docs_dir + 75] = 0x20;
    data[docs_dir + 90..docs_dir + 92].copy_from_slice(&7u16.to_le_bytes());
    data[docs_dir + 92..docs_dir + 96].copy_from_slice(&18u32.to_le_bytes());

    // FAT entry 7: End of chain
    data[fat_start + 28..fat_start + 32].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // === File Contents ===

    // Cluster 4-5: TEST.TXT content
    let test_file = 66 * 512;
    data[test_file..test_file + 13].copy_from_slice(b"Hello, FAT32!");

    // Cluster 6: README.MD content (19 bytes)
    let readme_file = 68 * 512;
    data[readme_file..readme_file + 19].copy_from_slice(b"# FAT32 Test Image\n");

    // Cluster 7: INFO.TXT content
    let info_file = 69 * 512;
    data[info_file..info_file + 18].copy_from_slice(b"Info file content\n");

    data
}

#[test]
fn test_boot_sector_parsing() {
    let image = create_test_image();
    let fs = Fat32::new(&image).expect("Should parse valid image");

    assert_eq!(fs.bytes_per_sector(), 512);
    assert_eq!(fs.root_cluster(), 2);
    assert_eq!(fs.boot_sector().sectors_per_cluster, 1);
    assert_eq!(fs.boot_sector().fat_count, 2);
}

#[test]
fn test_root_directory_listing() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    let entries = fs.read_directory(fs.root_cluster());

    // Should have TEST.TXT, DOCS, and README.MD
    assert_eq!(entries.len(), 3);

    let names: Vec<String> = entries.iter().map(|e| e.display_name()).collect();
    assert!(names.contains(&String::from("TEST.TXT")));
    assert!(names.contains(&String::from("DOCS")));
    assert!(names.contains(&String::from("README.MD")));
}

#[test]
fn test_find_file() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    // Case insensitive search
    let entry = fs.find_entry(fs.root_cluster(), "test.txt");
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.display_name(), "TEST.TXT");
    assert!(!entry.is_directory());
    assert_eq!(entry.size, 13);
}

#[test]
fn test_find_directory() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    let entry = fs.find_entry(fs.root_cluster(), "DOCS");
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert!(entry.is_directory());
}

#[test]
fn test_read_subdirectory() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    // Find DOCS directory
    let docs = fs.find_entry(fs.root_cluster(), "DOCS").unwrap();

    // Read its contents
    let entries = fs.read_directory(docs.cluster());

    // Should have ., .., and INFO.TXT
    let names: Vec<String> = entries.iter().map(|e| e.display_name()).collect();
    assert!(names.contains(&String::from(".")));
    assert!(names.contains(&String::from("..")));
    assert!(names.contains(&String::from("INFO.TXT")));
}

#[test]
fn test_read_file_content() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    let entry = fs.find_entry(fs.root_cluster(), "TEST.TXT").unwrap();
    let content = fs.read_file(&entry);

    assert_eq!(content.len(), 13);
    assert_eq!(&content, b"Hello, FAT32!");
}

#[test]
fn test_read_file_in_subdirectory() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    // Navigate to DOCS
    let docs = fs.find_entry(fs.root_cluster(), "DOCS").unwrap();

    // Find INFO.TXT
    let info = fs.find_entry(docs.cluster(), "INFO.TXT").unwrap();
    let content = fs.read_file(&info);

    assert_eq!(&content, b"Info file content\n");
}

#[test]
fn test_resolve_path() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    // Absolute path
    let entry = fs.resolve_path("/DOCS/INFO.TXT", fs.root_cluster());
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().display_name(), "INFO.TXT");

    // Non-existent path
    let entry = fs.resolve_path("/NONEXISTENT/FILE.TXT", fs.root_cluster());
    assert!(entry.is_none());
}

#[test]
fn test_file_not_found() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    let entry = fs.find_entry(fs.root_cluster(), "NOTFOUND.TXT");
    assert!(entry.is_none());
}

#[test]
fn test_invalid_image() {
    // Too small
    let small = vec![0u8; 100];
    assert!(Fat32::new(&small).is_none());

    // No valid signature
    let mut invalid = vec![0u8; 1024];
    invalid[510] = 0x00;
    invalid[511] = 0x00;
    assert!(Fat32::new(&invalid).is_none());
}

#[test]
fn test_fat_entry_types() {
    assert!(matches!(FatEntry::from_raw(0x00000000), FatEntry::Free));
    assert!(matches!(FatEntry::from_raw(0x00000001), FatEntry::Reserved));
    assert!(matches!(FatEntry::from_raw(0x00000100), FatEntry::Data(256)));
    assert!(matches!(FatEntry::from_raw(0x0FFFFFF7), FatEntry::BadCluster));
    assert!(matches!(FatEntry::from_raw(0x0FFFFFF8), FatEntry::EndOfChain));
    assert!(matches!(FatEntry::from_raw(0x0FFFFFFF), FatEntry::EndOfChain));
}

#[test]
fn test_cluster_chain() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    // TEST.TXT spans clusters 4 and 5
    let entry = fs.find_entry(fs.root_cluster(), "TEST.TXT").unwrap();
    let data = fs.read_cluster_chain(entry.cluster());

    // Should have data from 2 clusters (1 sector each = 512 bytes per cluster)
    assert!(data.len() >= 512);
}

#[test]
fn test_directory_attributes() {
    let image = create_test_image();
    let fs = Fat32::new(&image).unwrap();

    let entries = fs.read_directory(fs.root_cluster());

    for entry in entries {
        if entry.display_name() == "DOCS" {
            assert!(entry.is_directory());
            assert!(!entry.is_hidden());
        } else {
            assert!(!entry.is_directory());
        }
    }
}
