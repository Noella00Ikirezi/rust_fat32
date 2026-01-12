//! FAT32 Shell - Programme de démonstration

use std::io::{self, Write, BufRead};
use fat32_exam::fat32::Fat32;
use fat32_exam::shell::{ShellState, Output, Command, parse_command};
use fat32_exam::shell::{cmd_ls, cmd_cd, cmd_cat, cmd_more, cmd_pwd, cmd_help};

struct ConsoleOutput;

impl Output for ConsoleOutput {
    fn write_str(&mut self, s: &str) {
        print!("{}", s);
        io::stdout().flush().unwrap();
    }

    fn write_line(&mut self, s: &str) {
        println!("{}", s);
    }
}

/// Crée une image FAT32 de démonstration
fn create_demo_image() -> Vec<u8> {
    let mut data = vec![0u8; 1024 * 1024];

    // Boot sector
    data[11] = 0x00; data[12] = 0x02;
    data[13] = 1;
    data[14] = 32; data[15] = 0;
    data[16] = 2;
    data[32..36].copy_from_slice(&2048u32.to_le_bytes());
    data[36..40].copy_from_slice(&16u32.to_le_bytes());
    data[44..48].copy_from_slice(&2u32.to_le_bytes());
    data[510] = 0x55; data[511] = 0xAA;

    // FAT table
    let fat_start = 32 * 512;
    data[fat_start..fat_start + 4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());
    data[fat_start + 4..fat_start + 8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    data[fat_start + 8..fat_start + 12].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    data[fat_start + 12..fat_start + 16].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    data[fat_start + 16..fat_start + 20].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    data[fat_start + 20..fat_start + 24].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    data[fat_start + 24..fat_start + 28].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    // Root directory
    let root_dir = 64 * 512;

    // HELLO.TXT
    data[root_dir..root_dir + 8].copy_from_slice(b"HELLO   ");
    data[root_dir + 8..root_dir + 11].copy_from_slice(b"TXT");
    data[root_dir + 11] = 0x20;
    data[root_dir + 26..root_dir + 28].copy_from_slice(&4u16.to_le_bytes());
    data[root_dir + 28..root_dir + 32].copy_from_slice(&45u32.to_le_bytes());

    // DOCS directory
    data[root_dir + 32..root_dir + 40].copy_from_slice(b"DOCS    ");
    data[root_dir + 40..root_dir + 43].copy_from_slice(b"   ");
    data[root_dir + 43] = 0x10;
    data[root_dir + 58..root_dir + 60].copy_from_slice(&3u16.to_le_bytes());

    // README.MD
    data[root_dir + 64..root_dir + 72].copy_from_slice(b"README  ");
    data[root_dir + 72..root_dir + 75].copy_from_slice(b"MD ");
    data[root_dir + 75] = 0x20;
    data[root_dir + 90..root_dir + 92].copy_from_slice(&5u16.to_le_bytes());
    data[root_dir + 92..root_dir + 96].copy_from_slice(&89u32.to_le_bytes());

    // DOCS directory content
    let docs_dir = 65 * 512;
    data[docs_dir..docs_dir + 8].copy_from_slice(b".       ");
    data[docs_dir + 8..docs_dir + 11].copy_from_slice(b"   ");
    data[docs_dir + 11] = 0x10;
    data[docs_dir + 26..docs_dir + 28].copy_from_slice(&3u16.to_le_bytes());

    data[docs_dir + 32..docs_dir + 40].copy_from_slice(b"..      ");
    data[docs_dir + 40..docs_dir + 43].copy_from_slice(b"   ");
    data[docs_dir + 43] = 0x10;
    data[docs_dir + 58..docs_dir + 60].copy_from_slice(&0u16.to_le_bytes());

    data[docs_dir + 64..docs_dir + 72].copy_from_slice(b"INFO    ");
    data[docs_dir + 72..docs_dir + 75].copy_from_slice(b"TXT");
    data[docs_dir + 75] = 0x20;
    data[docs_dir + 90..docs_dir + 92].copy_from_slice(&6u16.to_le_bytes());
    data[docs_dir + 92..docs_dir + 96].copy_from_slice(&42u32.to_le_bytes());

    // File contents
    let hello_content = b"Hello! This is a test file for FAT32 shell.\n";
    let hello_file = 66 * 512;
    data[hello_file..hello_file + hello_content.len()].copy_from_slice(hello_content);

    let readme_content = b"# FAT32 Filesystem Demo\n\nThis is a demo FAT32 image.\nCreated for ESGI 4A Rust course.\n";
    let readme_file = 67 * 512;
    data[readme_file..readme_file + readme_content.len()].copy_from_slice(readme_content);

    let info_content = b"Info file inside DOCS directory.\nTest OK!\n";
    let info_file = 68 * 512;
    data[info_file..info_file + info_content.len()].copy_from_slice(info_content);

    data
}

fn main() {
    println!("========================================");
    println!("   FAT32 Filesystem Shell v0.1.0");
    println!("   Noella IKIREZI - ESGI 4A");
    println!("========================================");
    println!();

    let disk_data = create_demo_image();

    let fs = match Fat32::new(&disk_data) {
        Some(fs) => fs,
        None => {
            eprintln!("Error: Failed to parse FAT32 image");
            return;
        }
    };

    println!("FAT32 image loaded successfully!");
    println!("Type 'help' for available commands, 'exit' to quit.");
    println!();

    let mut state = ShellState::new(fs.root_cluster());
    let mut output = ConsoleOutput;
    let stdin = io::stdin();

    loop {
        print!("{}> ", state.pwd());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                continue;
            }
        }

        match parse_command(&input) {
            Command::Ls(path) => cmd_ls(&fs, &state, path, &mut output),
            Command::Cd(path) => cmd_cd(&fs, &mut state, path, &mut output),
            Command::Cat(file) => cmd_cat(&fs, &state, file, &mut output),
            Command::More(file) => cmd_more(&fs, &state, file, &mut output, 20),
            Command::Pwd => cmd_pwd(&state, &mut output),
            Command::Help => cmd_help(&mut output),
            Command::Exit => {
                println!("Goodbye!");
                break;
            }
            Command::Unknown(cmd) => {
                println!("Unknown command: {}", cmd);
                println!("Type 'help' for available commands.");
            }
            Command::Empty => {}
        }
        println!();
    }
}
