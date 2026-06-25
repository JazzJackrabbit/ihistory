use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub command: String,
}

fn aliases_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?.join("ihistory");
    fs::create_dir_all(&config_dir).ok()?;
    Some(config_dir.join("view aliases"))
}

pub fn load_aliases() -> Vec<Alias> {
    let Some(path) = aliases_path() else {
        return Vec::new();
    };
    let Ok(file) = File::open(&path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| {
            let eq_pos = line.find('=')?;
            let name = line[..eq_pos].to_string();
            let command = line[eq_pos + 1..].replace('\0', "\n");
            if name.is_empty() {
                return None;
            }
            Some(Alias { name, command })
        })
        .collect()
}

pub fn save_aliases(aliases: &[Alias]) -> std::io::Result<()> {
    let path = aliases_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config directory",
        )
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    for alias in aliases {
        let encoded = alias.command.replace('\n', "\0");
        writeln!(file, "{}={}", alias.name, encoded)?;
    }
    Ok(())
}

pub fn filter_aliases(aliases: &[Alias], query: &str) -> Vec<(usize, Alias)> {
    if query.is_empty() {
        return aliases
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.clone()))
            .collect();
    }
    let query_lower = query.to_lowercase();
    aliases
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.name.to_lowercase().contains(&query_lower)
                || a.command.to_lowercase().contains(&query_lower)
        })
        .map(|(i, a)| (i, a.clone()))
        .collect()
}

pub fn validate_alias_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Name cannot be empty");
    }
    if name.len() > 32 {
        return Err("Name too long (max 32 chars)");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Only a-z, A-Z, 0-9, _, - allowed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_alias_name_valid() {
        assert!(validate_alias_name("deploy").is_ok());
        assert!(validate_alias_name("my-alias").is_ok());
        assert!(validate_alias_name("test_123").is_ok());
    }

    #[test]
    fn test_validate_alias_name_empty() {
        assert!(validate_alias_name("").is_err());
    }

    #[test]
    fn test_validate_alias_name_too_long() {
        let long_name = "a".repeat(33);
        assert!(validate_alias_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_alias_name_invalid_chars() {
        assert!(validate_alias_name("my alias").is_err());
        assert!(validate_alias_name("foo=bar").is_err());
        assert!(validate_alias_name("hello!").is_err());
    }
}
