//! Parser de commandes pour le shell FAT32

/// Commande parsée
#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    Ls(Option<&'a str>),
    Cd(&'a str),
    Cat(&'a str),
    More(&'a str),
    Pwd,
    Help,
    Exit,
    Unknown(&'a str),
    Empty,
}

/// Parse une chaîne de commande
pub fn parse_command(input: &str) -> Command<'_> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Command::Empty;
    }

    let mut parts = trimmed.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg = parts.next().map(|s| s.trim());

    match cmd.to_ascii_lowercase().as_str() {
        "ls" | "dir" | "list" => Command::Ls(arg),

        "cd" | "chdir" => match arg {
            Some(path) if !path.is_empty() => Command::Cd(path),
            _ => Command::Cd("/"),
        },

        "cat" | "type" | "read" => match arg {
            Some(filename) if !filename.is_empty() => Command::Cat(filename),
            _ => Command::Empty,
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

/// Parse un chemin en composants
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
