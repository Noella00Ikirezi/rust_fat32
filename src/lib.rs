//! Implémentation FAT32 - Compatible no_std pour ESGI 4A
//!
//! Fonctionnalités: parsing boot sector, navigation répertoires, lecture fichiers, shell interactif

// Pour no_std, décommenter:
// #![no_std]
// #![feature(alloc_error_handler)]

#![allow(static_mut_refs)]

extern crate alloc;

pub mod fat32;
pub mod shell;
pub mod allocator;

// Handlers no_std (décommenter pour la soumission):
// use core::panic::PanicInfo;
// #[panic_handler]
// fn panic(_info: &PanicInfo) -> ! { loop {} }
// #[alloc_error_handler]
// fn alloc_error(_layout: core::alloc::Layout) -> ! { loop {} }

pub use fat32::{Fat32, DirEntry, BootSector};
pub use shell::{ShellState, Command, Output};

pub const VERSION: &str = "0.1.0";

/// Affiche les infos de la bibliothèque
pub fn print_info<O: Output>(out: &mut O) {
    out.write_line("FAT32 Filesystem Implementation");
    out.write_line(&alloc::format!("Version: {}", VERSION));
    out.write_line("Author: Noella IKIREZI - ESGI 4A");
}
