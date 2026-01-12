//! Module Shell - Interface en ligne de commande pour naviguer le filesystem FAT32

pub mod parser;
pub mod commands;

pub use parser::{Command, parse_command};
pub use commands::{ShellState, Output, cmd_ls, cmd_cd, cmd_cat, cmd_more, cmd_pwd, cmd_help};

use crate::fat32::Fat32;

/// Boucle principale du shell interactif
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
        out.write_str(&format!("{}> ", state.pwd()));

        let input = match get_input() {
            Some(s) => s,
            None => break,
        };

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

/// Exécute une seule commande (pour usage non-interactif)
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
