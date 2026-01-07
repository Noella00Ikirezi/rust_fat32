//! Shell Module for FAT32 Filesystem
//!
//! Provides a command-line interface for navigating and reading
//! FAT32 filesystems.
//!
//! # Commands
//! - `ls` - List directory contents
//! - `cd` - Change directory
//! - `cat` - Display file contents
//! - `more` - Display file with pagination
//! - `pwd` - Print working directory
//! - `help` - Show help
//! - `exit` - Exit shell

pub mod parser;
pub mod commands;

pub use parser::{Command, parse_command};
pub use commands::{ShellState, Output, cmd_ls, cmd_cd, cmd_cat, cmd_more, cmd_pwd, cmd_help};

use crate::fat32::Fat32;

/// Main shell loop
///
/// Runs an interactive shell for the given filesystem.
/// This is a template - actual input handling depends on your platform.
///
/// # Arguments
/// * `fs` - FAT32 filesystem to operate on
/// * `out` - Output device
/// * `get_input` - Function to get user input
///
/// # Example
/// ```ignore
/// run_shell(&fs, &mut output, || {
///     // Read line from keyboard
///     read_line()
/// });
/// ```
pub fn run_shell<O, F>(fs: &Fat32, out: &mut O, mut get_input: F)
where
    O: Output,
    F: FnMut() -> Option<alloc::string::String>,
{
    extern crate alloc;
    use alloc::format;

    let mut state = ShellState::new(fs.root_cluster());

    out.write_line("FAT32 Shell - Type 'help' for commands");
    out.write_line("");

    loop {
        // Show prompt
        out.write_str(&format!("{}> ", state.pwd()));

        // Get input
        let input = match get_input() {
            Some(s) => s,
            None => break, // EOF or error
        };

        // Parse and execute command
        match parse_command(&input) {
            Command::Ls(path) => cmd_ls(fs, &state, path, out),
            Command::Cd(path) => cmd_cd(fs, &mut state, path, out),
            Command::Cat(file) => cmd_cat(fs, &state, file, out),
            Command::More(file) => cmd_more(fs, &state, file, out, 20),
            Command::Pwd => cmd_pwd(&state, out),
            Command::Help => cmd_help(out),
            Command::Exit => {
                out.write_line("Goodbye!");
                break;
            }
            Command::Unknown(cmd) => {
                out.write_line(&format!("Unknown command: {}", cmd));
                out.write_line("Type 'help' for available commands");
            }
            Command::Empty => {}
        }

        out.write_line("");
    }
}

/// Execute a single command
///
/// For non-interactive use or scripting.
///
/// # Arguments
/// * `fs` - FAT32 filesystem
/// * `state` - Shell state (modified by cd)
/// * `input` - Command string
/// * `out` - Output device
///
/// # Returns
/// `false` if exit command was given, `true` otherwise
pub fn execute_command<O: Output>(
    fs: &Fat32,
    state: &mut ShellState,
    input: &str,
    out: &mut O,
) -> bool {
    extern crate alloc;
    use alloc::format;

    match parse_command(input) {
        Command::Ls(path) => {
            cmd_ls(fs, state, path, out);
            true
        }
        Command::Cd(path) => {
            cmd_cd(fs, state, path, out);
            true
        }
        Command::Cat(file) => {
            cmd_cat(fs, state, file, out);
            true
        }
        Command::More(file) => {
            cmd_more(fs, state, file, out, 20);
            true
        }
        Command::Pwd => {
            cmd_pwd(state, out);
            true
        }
        Command::Help => {
            cmd_help(out);
            true
        }
        Command::Exit => false,
        Command::Unknown(cmd) => {
            out.write_line(&format!("Unknown command: {}", cmd));
            true
        }
        Command::Empty => true,
    }
}
