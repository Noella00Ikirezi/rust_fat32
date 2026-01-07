//! Shell Commands Implementation
//!
//! Implements ls, cd, cat, more, pwd, and help commands.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::fat32::Fat32;

/// Shell state tracking current directory
pub struct ShellState {
    /// Current directory cluster
    pub current_cluster: u32,
    /// Path components from root
    pub current_path: Vec<String>,
}

impl ShellState {
    /// Create new shell state at root directory
    pub fn new(root_cluster: u32) -> Self {
        ShellState {
            current_cluster: root_cluster,
            current_path: Vec::new(),
        }
    }

    /// Get current working directory as string
    pub fn pwd(&self) -> String {
        if self.current_path.is_empty() {
            String::from("/")
        } else {
            format!("/{}", self.current_path.join("/"))
        }
    }

    /// Check if at root directory
    pub fn is_root(&self) -> bool {
        self.current_path.is_empty()
    }
}

/// Output trait for writing to display
///
/// Implement this trait for your specific hardware/output device.
pub trait Output {
    /// Write string (no newline)
    fn write_str(&mut self, s: &str);

    /// Write string with newline
    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.write_str("\n");
    }

    /// Write formatted string
    fn write_fmt(&mut self, s: &str) {
        self.write_str(s);
    }
}

/// Simple string buffer output (for testing)
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

/// Execute ls command - list directory contents
///
/// # Arguments
/// * `fs` - FAT32 filesystem
/// * `state` - Current shell state
/// * `path` - Optional path to list (None = current directory)
/// * `out` - Output device
pub fn cmd_ls<O: Output>(
    fs: &Fat32,
    state: &ShellState,
    path: Option<&str>,
    out: &mut O,
) {
    // Determine which cluster to list
    let cluster = match path {
        Some(p) if !p.is_empty() => {
            // Navigate to specified path
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

    // Calculate totals
    let mut total_files = 0u32;
    let mut total_dirs = 0u32;
    let mut total_size = 0u64;

    for (entry, long_name) in &entries {
        if entry.is_hidden() {
            continue;
        }

        // Get display name (prefer long name)
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

/// Execute cd command - change directory
///
/// # Arguments
/// * `fs` - FAT32 filesystem
/// * `state` - Shell state to modify
/// * `path` - Path to change to
/// * `out` - Output device
pub fn cmd_cd<O: Output>(
    fs: &Fat32,
    state: &mut ShellState,
    path: &str,
    out: &mut O,
) {
    match path {
        // Go to root
        "/" | "" => {
            state.current_path.clear();
            state.current_cluster = fs.root_cluster();
        }

        // Go up one level
        ".." => {
            if state.current_path.pop().is_some() {
                // Recalculate cluster by navigating from root
                state.current_cluster = navigate_from_root(fs, &state.current_path);
            }
            // If already at root, do nothing
        }

        // Current directory (no-op)
        "." => {}

        // Navigate to path
        name => {
            if let Some((cluster, is_dir)) = resolve_to_cluster(fs, state, name) {
                if is_dir {
                    // Update state based on absolute vs relative path
                    if name.starts_with('/') {
                        // Absolute path - rebuild path from components
                        state.current_path.clear();
                        for component in name.split('/').filter(|s| !s.is_empty()) {
                            if component != ".." {
                                state.current_path.push(String::from(component));
                            } else if !state.current_path.is_empty() {
                                state.current_path.pop();
                            }
                        }
                    } else {
                        // Relative path
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

/// Execute cat command - display file contents
///
/// # Arguments
/// * `fs` - FAT32 filesystem
/// * `state` - Current shell state
/// * `filename` - File to display
/// * `out` - Output device
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

            // Try to display as text
            if let Ok(text) = core::str::from_utf8(&data) {
                out.write_str(text);
                if !text.is_empty() && !text.ends_with('\n') {
                    out.write_str("\n");
                }
            } else {
                // Binary file - show hex dump
                hex_dump(&data, out, 256);
            }
        }
        None => {
            out.write_line("File not found");
        }
    }
}

/// Execute more command - display file with pagination
///
/// # Arguments
/// * `fs` - FAT32 filesystem
/// * `state` - Current shell state
/// * `filename` - File to display
/// * `out` - Output device
/// * `lines_per_page` - Number of lines per page
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
                        // In actual implementation, wait for keypress here
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

/// Execute pwd command - print working directory
pub fn cmd_pwd<O: Output>(state: &ShellState, out: &mut O) {
    out.write_line(&state.pwd());
}

/// Execute help command - show available commands
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

// Helper functions

/// Navigate from root using path components
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

/// Resolve path to cluster number
///
/// Returns (cluster, is_directory) or None if not found
fn resolve_to_cluster(fs: &Fat32, state: &ShellState, path: &str) -> Option<(u32, bool)> {
    let (is_absolute, components) = super::parser::parse_path(path);

    let mut cluster = if is_absolute {
        fs.root_cluster()
    } else {
        state.current_cluster
    };

    for (i, component) in components.iter().enumerate() {
        match *component {
            ".." => {
                // For simplicity, we'd need parent tracking
                // This is a simplified version
                continue;
            }
            "." => continue,
            name => {
                if let Some(entry) = fs.find_entry(cluster, name) {
                    if i == components.len() - 1 {
                        // Last component
                        let new_cluster = if entry.cluster() == 0 {
                            fs.root_cluster() // Handle root references
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
                        return None; // Can't traverse through file
                    }
                } else {
                    return None;
                }
            }
        }
    }

    // If we get here with no components, return current cluster
    Some((cluster, true))
}

/// Display hex dump of binary data
fn hex_dump<O: Output>(data: &[u8], out: &mut O, max_bytes: usize) {
    let display_len = data.len().min(max_bytes);

    for (i, chunk) in data[..display_len].chunks(16).enumerate() {
        // Address
        let mut line = format!("{:08X}:  ", i * 16);

        // Hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            line.push_str(&format!("{:02X} ", byte));
            if j == 7 {
                line.push(' ');
            }
        }

        // Padding if needed
        for j in chunk.len()..16 {
            line.push_str("   ");
            if j == 7 {
                line.push(' ');
            }
        }

        line.push_str(" |");

        // ASCII representation
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
