//! FAT32 Filesystem Implementation
//!
//! A no_std compatible FAT32 filesystem implementation for the ESGI 4A exam.
//!
//! # Features
//! - Read FAT32 boot sector and filesystem parameters
//! - Navigate directory structure
//! - Read files
//! - Interactive shell with ls, cd, cat, more, pwd commands
//!
//! # Requirements
//! - `no_std` environment
//! - `alloc` crate for heap allocation
//!
//! # Usage
//! ```ignore
//! use fat32_exam::fat32::Fat32;
//! use fat32_exam::shell::{ShellState, Output, cmd_ls};
//!
//! // Load disk image
//! let disk_data: &[u8] = /* ... */;
//!
//! // Create filesystem
//! let fs = Fat32::new(disk_data).expect("Invalid FAT32 image");
//!
//! // Read root directory
//! let entries = fs.read_directory(fs.root_cluster());
//! for entry in entries {
//!     println!("{}", entry.display_name());
//! }
//! ```
//!
//! # Author
//! Noella IKIREZI - ESGI 4A
//!
//! # References
//! - Microsoft FAT32 File System Specification
//! - https://os.phil-opp.com/
//! - https://rust-unofficial.github.io/too-many-lists/

// ============================================================================
// CONFIGURATION NO_STD
// ============================================================================
// Pour la SOUMISSION finale (no_std), décommenter ces lignes:
// #![no_std]
// #![feature(alloc_error_handler)]
//
// Et commenter ou supprimer les tests qui utilisent std
// ============================================================================

#![allow(static_mut_refs)]

extern crate alloc;

pub mod fat32;
pub mod shell;
pub mod allocator;

// ============================================================================
// HANDLERS NO_STD - Décommenter pour la soumission
// ============================================================================
// use core::panic::PanicInfo;
//
// #[panic_handler]
// fn panic(_info: &PanicInfo) -> ! {
//     loop {}
// }
//
// #[alloc_error_handler]
// fn alloc_error(_layout: core::alloc::Layout) -> ! {
//     loop {}
// }
// ============================================================================

// Re-export commonly used types at crate root
pub use fat32::{Fat32, DirEntry, BootSector};
pub use shell::{ShellState, Command, Output};

/// Library version
pub const VERSION: &str = "0.1.0";

/// Print library info
pub fn print_info<O: Output>(out: &mut O) {
    out.write_line("FAT32 Filesystem Implementation");
    out.write_line(&alloc::format!("Version: {}", VERSION));
    out.write_line("Author: Noella IKIREZI - ESGI 4A");
    out.write_line("");
    out.write_line("Features:");
    out.write_line("  - Boot sector parsing");
    out.write_line("  - FAT table reading");
    out.write_line("  - Directory navigation");
    out.write_line("  - File reading");
    out.write_line("  - Interactive shell");
}
