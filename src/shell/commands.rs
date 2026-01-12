//! Implémentation des commandes shell: ls, cd, cat, more, pwd, help

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::fat32::Fat32;

/// État du shell avec le répertoire courant
pub struct ShellState {
    pub current_cluster: u32,
    pub current_path: Vec<String>,
}

impl ShellState {
    /// Crée un nouvel état au répertoire racine
    pub fn new(root_cluster: u32) -> Self {
        ShellState {
            current_cluster: root_cluster,
            current_path: Vec::new(),
        }
    }

    /// Retourne le chemin courant
    pub fn pwd(&self) -> String {
        if self.current_path.is_empty() {
            String::from("/")
        } else {
            format!("/{}", self.current_path.join("/"))
        }
    }

    /// Vérifie si on est à la racine
    pub fn is_root(&self) -> bool {
        self.current_path.is_empty()
    }
}

/// Trait pour l'affichage
pub trait Output {
    fn write_str(&mut self, s: &str);

    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.write_str("\n");
    }

    fn write_fmt(&mut self, s: &str) {
        self.write_str(s);
    }
}

#[cfg(test)]
pub struct StringOutput {
    pub buffer: String,
}

#[cfg(test)]
impl StringOutput {
    pub fn new() -> Self {
        StringOutput { buffer: String::new() }
    }
}

#[cfg(test)]
impl Output for StringOutput {
    fn write_str(&mut self, s: &str) {
        self.buffer.push_str(s);
    }
}

/// Commande ls - liste le contenu d'un répertoire
pub fn cmd_ls<O: Output>(
    fs: &Fat32,
    state: &ShellState,
    path: Option<&str>,
    out: &mut O,
) {
    let cluster = match path {
        Some(p) if !p.is_empty() => {
            match resolve_to_cluster(fs, state, p) {
                Some((c, true)) => c,
                Some((_, false)) => {
                    out.write_line("Not a directory");
                    return;
                }
                None => {
                    out.write_line("Path not found");
                    return;
                }
            }
        }
        _ => state.current_cluster,
    };

    let entries = fs.read_directory_with_lfn(cluster);

    if entries.is_empty() {
        out.write_line("(empty directory)");
        return;
    }

    let mut total_files = 0u32;
    let mut total_dirs = 0u32;
    let mut total_size = 0u64;

    for (entry, long_name) in &entries {
        if entry.is_hidden() {
            continue;
        }

        let name = long_name.as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| "");
        let name = if name.is_empty() {
            entry.display_name()
        } else {
            String::from(name)
        };

        if entry.is_directory() {
            out.write_line(&format!("  <DIR>       {}/", name));
            total_dirs += 1;
        } else {
            out.write_line(&format!("{:>10}    {}", entry.size, name));
            total_files += 1;
            total_size += entry.size as u64;
        }
    }

    out.write_line("");
    out.write_line(&format!("  {} file(s)  {} bytes", total_files, total_size));
    out.write_line(&format!("  {} dir(s)", total_dirs));
}

/// Commande cd - change de répertoire
pub fn cmd_cd<O: Output>(
    fs: &Fat32,
    state: &mut ShellState,
    path: &str,
    out: &mut O,
) {
    match path {
        "/" | "" => {
            state.current_path.clear();
            state.current_cluster = fs.root_cluster();
        }

        ".." => {
            if state.current_path.pop().is_some() {
                state.current_cluster = navigate_from_root(fs, &state.current_path);
            }
        }

        "." => {}

        name => {
            if let Some((cluster, is_dir)) = resolve_to_cluster(fs, state, name) {
                if is_dir {
                    if name.starts_with('/') {
                        state.current_path.clear();
                        for component in name.split('/').filter(|s| !s.is_empty()) {
                            if component != ".." {
                                state.current_path.push(String::from(component));
                            } else if !state.current_path.is_empty() {
                                state.current_path.pop();
                            }
                        }
                    } else {
                        for component in name.split('/').filter(|s| !s.is_empty()) {
                            if component == ".." {
                                state.current_path.pop();
                            } else if component != "." {
                                state.current_path.push(String::from(component));
                            }
                        }
                    }
                    state.current_cluster = cluster;
                } else {
                    out.write_line("Not a directory");
                }
            } else {
                out.write_line("Directory not found");
            }
        }
    }
}

/// Commande cat - affiche le contenu d'un fichier
pub fn cmd_cat<O: Output>(
    fs: &Fat32,
    state: &ShellState,
    filename: &str,
    out: &mut O,
) {
    let entry = if filename.contains('/') {
        fs.resolve_path(filename, state.current_cluster)
    } else {
        fs.find_entry(state.current_cluster, filename)
    };

    match entry {
        Some(ref e) if e.is_directory() => {
            out.write_line("Cannot cat a directory");
        }
        Some(ref e) => {
            let data = fs.read_file(e);

            if let Ok(text) = core::str::from_utf8(&data) {
                out.write_str(text);
                if !text.is_empty() && !text.ends_with('\n') {
                    out.write_str("\n");
                }
            } else {
                hex_dump(&data, out, 256);
            }
        }
        None => {
            out.write_line("File not found");
        }
    }
}

/// Commande more - affiche un fichier avec pagination
pub fn cmd_more<O: Output>(
    fs: &Fat32,
    state: &ShellState,
    filename: &str,
    out: &mut O,
    lines_per_page: usize,
) {
    let entry = if filename.contains('/') {
        fs.resolve_path(filename, state.current_cluster)
    } else {
        fs.find_entry(state.current_cluster, filename)
    };

    match entry {
        Some(ref e) if e.is_directory() => {
            out.write_line("Cannot display a directory");
        }
        Some(ref e) => {
            let data = fs.read_file(e);

            if let Ok(text) = core::str::from_utf8(&data) {
                let mut line_count = 0;

                for line in text.lines() {
                    out.write_line(line);
                    line_count += 1;

                    if line_count >= lines_per_page {
                        out.write_line("-- More (press any key to continue) --");
                        line_count = 0;
                    }
                }
            } else {
                out.write_line("Binary file - use cat for hex dump");
            }
        }
        None => {
            out.write_line("File not found");
        }
    }
}

/// Commande pwd - affiche le répertoire courant
pub fn cmd_pwd<O: Output>(state: &ShellState, out: &mut O) {
    out.write_line(&state.pwd());
}

/// Commande help - affiche l'aide
pub fn cmd_help<O: Output>(out: &mut O) {
    out.write_line("FAT32 Shell Commands:");
    out.write_line("");
    out.write_line("  ls [path]     - List directory contents");
    out.write_line("  cd <dir>      - Change directory");
    out.write_line("  cat <file>    - Display file contents");
    out.write_line("  more <file>   - Display file with pagination");
    out.write_line("  pwd           - Print working directory");
    out.write_line("  help          - Show this help");
    out.write_line("  exit          - Exit shell");
    out.write_line("");
    out.write_line("Path examples:");
    out.write_line("  cd /          - Go to root");
    out.write_line("  cd ..         - Go up one level");
    out.write_line("  cd Documents  - Enter subdirectory");
    out.write_line("  cat /path/to/file.txt - Read file by path");
}

/// Navigate depuis la racine avec les composants du chemin
fn navigate_from_root(fs: &Fat32, path: &[String]) -> u32 {
    let mut cluster = fs.root_cluster();

    for component in path {
        if let Some(entry) = fs.find_entry(cluster, component) {
            if entry.is_directory() {
                cluster = entry.cluster();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    cluster
}

/// Résout un chemin vers un numéro de cluster
fn resolve_to_cluster(fs: &Fat32, state: &ShellState, path: &str) -> Option<(u32, bool)> {
    let (is_absolute, components) = super::parser::parse_path(path);

    let mut cluster = if is_absolute {
        fs.root_cluster()
    } else {
        state.current_cluster
    };

    for (i, component) in components.iter().enumerate() {
        match *component {
            ".." => continue,
            "." => continue,
            name => {
                if let Some(entry) = fs.find_entry(cluster, name) {
                    if i == components.len() - 1 {
                        let new_cluster = if entry.cluster() == 0 {
                            fs.root_cluster()
                        } else {
                            entry.cluster()
                        };
                        return Some((new_cluster, entry.is_directory()));
                    } else if entry.is_directory() {
                        cluster = entry.cluster();
                        if cluster == 0 {
                            cluster = fs.root_cluster();
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
        }
    }

    Some((cluster, true))
}

/// Affiche un dump hexadécimal
fn hex_dump<O: Output>(data: &[u8], out: &mut O, max_bytes: usize) {
    let display_len = data.len().min(max_bytes);

    for (i, chunk) in data[..display_len].chunks(16).enumerate() {
        let mut line = format!("{:08X}:  ", i * 16);

        for (j, byte) in chunk.iter().enumerate() {
            line.push_str(&format!("{:02X} ", byte));
            if j == 7 {
                line.push(' ');
            }
        }

        for j in chunk.len()..16 {
            line.push_str("   ");
            if j == 7 {
                line.push(' ');
            }
        }

        line.push_str(" |");

        for byte in chunk {
            if *byte >= 0x20 && *byte <= 0x7E {
                line.push(*byte as char);
            } else {
                line.push('.');
            }
        }

        line.push('|');
        out.write_line(&line);
    }

    if data.len() > max_bytes {
        out.write_line(&format!("... ({} more bytes)", data.len() - max_bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_state() {
        let state = ShellState::new(2);
        assert!(state.is_root());
        assert_eq!(state.pwd(), "/");
    }

    #[test]
    fn test_pwd_with_path() {
        let mut state = ShellState::new(2);
        state.current_path.push(String::from("Documents"));
        state.current_path.push(String::from("Work"));

        assert_eq!(state.pwd(), "/Documents/Work");
        assert!(!state.is_root());
    }
}
