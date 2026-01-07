//! Command Parser for FAT32 Shell
//!
//! Parses user input into structured commands.

/// Parsed shell command
#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    /// List directory contents
    Ls(Option<&'a str>),
    /// Change directory
    Cd(&'a str),
    /// Display file contents
    Cat(&'a str),
    /// Display file with pagination
    More(&'a str),
    /// Print working directory
    Pwd,
    /// Show help
    Help,
    /// Exit shell
    Exit,
    /// Unknown command
    Unknown(&'a str),
    /// Empty input
    Empty,
}

/// Parse command string into Command enum
///
/// # Arguments
/// * `input` - Raw user input string
///
/// # Returns
/// Parsed Command variant
///
/// # Examples
/// ```
/// use fat32_exam::shell::parser::parse_command;
/// use fat32_exam::shell::parser::Command;
///
/// assert!(matches!(parse_command("ls"), Command::Ls(None)));
/// assert!(matches!(parse_command("cd Documents"), Command::Cd("Documents")));
/// ```
pub fn parse_command(input: &str) -> Command<'_> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Command::Empty;
    }

    // Split into command and argument
    let mut parts = trimmed.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg = parts.next().map(|s| s.trim());

    // Match command (case-insensitive)
    match cmd.to_ascii_lowercase().as_str() {
        "ls" | "dir" | "list" => Command::Ls(arg),

        "cd" | "chdir" => match arg {
            Some(path) if !path.is_empty() => Command::Cd(path),
            _ => Command::Cd("/"), // cd with no args goes to root
        },

        "cat" | "type" | "read" => match arg {
            Some(filename) if !filename.is_empty() => Command::Cat(filename),
            _ => Command::Empty, // Need filename
        },

        "more" | "less" | "page" => match arg {
            Some(filename) if !filename.is_empty() => Command::More(filename),
            _ => Command::Empty,
        },

        "pwd" | "cwd" => Command::Pwd,

        "help" | "?" | "h" => Command::Help,

        "exit" | "quit" | "q" => Command::Exit,

        _ => Command::Unknown(cmd),
    }
}

/// Parse path into components
///
/// # Arguments
/// * `path` - Path string (e.g., "/Documents/file.txt")
///
/// # Returns
/// Vector of path components
pub fn parse_path(path: &str) -> (bool, alloc::vec::Vec<&str>) {
    extern crate alloc;
    use alloc::vec::Vec;

    let is_absolute = path.starts_with('/');
    let components: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();

    (is_absolute, components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_command() {
        assert!(matches!(parse_command("ls"), Command::Ls(None)));
        assert!(matches!(parse_command("LS"), Command::Ls(None)));
        assert!(matches!(parse_command("dir"), Command::Ls(None)));

        if let Command::Ls(Some(path)) = parse_command("ls /Documents") {
            assert_eq!(path, "/Documents");
        } else {
            panic!("Expected Ls with path");
        }
    }

    #[test]
    fn test_cd_command() {
        if let Command::Cd(path) = parse_command("cd Documents") {
            assert_eq!(path, "Documents");
        } else {
            panic!("Expected Cd");
        }

        if let Command::Cd(path) = parse_command("cd ..") {
            assert_eq!(path, "..");
        } else {
            panic!("Expected Cd");
        }

        // cd with no args -> root
        if let Command::Cd(path) = parse_command("cd") {
            assert_eq!(path, "/");
        } else {
            panic!("Expected Cd to root");
        }
    }

    #[test]
    fn test_cat_command() {
        if let Command::Cat(file) = parse_command("cat readme.txt") {
            assert_eq!(file, "readme.txt");
        } else {
            panic!("Expected Cat");
        }

        // cat without filename
        assert!(matches!(parse_command("cat"), Command::Empty));
    }

    #[test]
    fn test_special_commands() {
        assert!(matches!(parse_command("pwd"), Command::Pwd));
        assert!(matches!(parse_command("help"), Command::Help));
        assert!(matches!(parse_command("exit"), Command::Exit));
        assert!(matches!(parse_command("quit"), Command::Exit));
    }

    #[test]
    fn test_empty_and_unknown() {
        assert!(matches!(parse_command(""), Command::Empty));
        assert!(matches!(parse_command("   "), Command::Empty));

        if let Command::Unknown(cmd) = parse_command("foobar") {
            assert_eq!(cmd, "foobar");
        } else {
            panic!("Expected Unknown");
        }
    }
}
