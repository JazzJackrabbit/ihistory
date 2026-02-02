use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: Option<i64>,
    #[allow(dead_code)]
    pub raw_line: Option<String>,
}

fn blocklist_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?.join("ihistory");
    fs::create_dir_all(&config_dir).ok()?;
    Some(config_dir.join("deleted"))
}

fn load_blocklist() -> HashSet<String> {
    let Some(path) = blocklist_path() else {
        return HashSet::new();
    };
    let Ok(file) = File::open(&path) else {
        return HashSet::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|line| line.replace('\0', "\n"))
        .collect()
}

fn add_to_blocklist(command: &str) -> std::io::Result<()> {
    let path = blocklist_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config directory",
        )
    })?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let encoded = command.replace('\n', "\0");
    writeln!(file, "{}", encoded)?;
    Ok(())
}

pub fn detect_history_file() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            let zsh_history = home.join(".zsh_history");
            if zsh_history.exists() {
                return Some(zsh_history);
            }
        } else if shell.contains("bash") {
            let bash_history = home.join(".bash_history");
            if bash_history.exists() {
                return Some(bash_history);
            }
        }
    }

    let zsh_history = home.join(".zsh_history");
    if zsh_history.exists() {
        return Some(zsh_history);
    }

    let bash_history = home.join(".bash_history");
    if bash_history.exists() {
        return Some(bash_history);
    }

    None
}

fn is_zsh_format(path: &Path) -> bool {
    path.to_string_lossy().contains("zsh")
}

fn is_self_command(cmd: &str) -> bool {
    cmd == "ih" || cmd == "ihistory" || cmd.starts_with("ih ") || cmd.starts_with("ihistory ")
}

struct ParsedZshLine {
    command: String,
    timestamp: Option<i64>,
    raw_line: String,
}

/// Parses zsh extended history format: `: EPOCH:DURATION;command`
fn parse_zsh_line(line: &str) -> Option<ParsedZshLine> {
    if let Some(rest) = line.strip_prefix(": ") {
        if let Some(semi_pos) = rest.find(';') {
            let meta = &rest[..semi_pos];
            let command = rest[semi_pos + 1..].to_string();

            if command.is_empty() {
                return None;
            }

            let timestamp = meta.split(':').next().and_then(|s| s.parse::<i64>().ok());

            return Some(ParsedZshLine {
                command,
                timestamp,
                raw_line: line.to_string(),
            });
        }
        // Malformed extended format - treat rest as command
        if !rest.is_empty() {
            return Some(ParsedZshLine {
                command: rest.to_string(),
                timestamp: None,
                raw_line: line.to_string(),
            });
        }
    } else if !line.is_empty() {
        return Some(ParsedZshLine {
            command: line.to_string(),
            timestamp: None,
            raw_line: line.to_string(),
        });
    }
    None
}

fn parse_bash_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

pub fn load_history(path: &Path, limit: usize) -> Result<Vec<HistoryEntry>, std::io::Error> {
    let content = fs::read(path)?;
    let is_zsh = is_zsh_format(path);

    let mut entry_data: HashMap<String, (Option<i64>, Option<String>)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut multiline_buffer: Option<(String, Option<i64>, String)> = None;

    for line_bytes in content.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line_bytes).into_owned();
        if is_zsh {
            if let Some((ref mut cmd, ref ts, ref mut raw)) = multiline_buffer {
                raw.push('\n');
                raw.push_str(&line);
                cmd.push('\n');
                cmd.push_str(&line);

                if !line.ends_with('\\') {
                    let command = cmd.clone();
                    let timestamp = *ts;
                    let raw_line = raw.clone();
                    multiline_buffer = None;

                    if entry_data.contains_key(&command) {
                        order.retain(|c| c != &command);
                    }
                    order.push(command.clone());
                    entry_data.insert(command, (timestamp, Some(raw_line)));
                }
                continue;
            }

            if let Some(parsed) = parse_zsh_line(&line) {
                if parsed.command.ends_with('\\') {
                    multiline_buffer = Some((parsed.command, parsed.timestamp, parsed.raw_line));
                } else {
                    if entry_data.contains_key(&parsed.command) {
                        order.retain(|c| c != &parsed.command);
                    }
                    order.push(parsed.command.clone());
                    entry_data.insert(parsed.command, (parsed.timestamp, Some(parsed.raw_line)));
                }
            }
        } else if let Some(command) = parse_bash_line(&line) {
            if entry_data.contains_key(&command) {
                order.retain(|c| c != &command);
            }
            order.push(command.clone());
            entry_data.insert(command, (None, Some(line.clone())));
        }
    }

    order.reverse();

    let blocklist = load_blocklist();

    let entries: Vec<HistoryEntry> = order
        .into_iter()
        .filter(|cmd| !blocklist.contains(cmd))
        .filter(|cmd| !is_self_command(cmd))
        .map(|command| {
            let (timestamp, raw_line) = entry_data.remove(&command).unwrap_or((None, None));
            HistoryEntry {
                command,
                timestamp,
                raw_line,
            }
        })
        .collect();

    let entries = if limit > 0 {
        entries.into_iter().take(limit).collect()
    } else {
        entries
    };

    Ok(entries)
}

pub fn delete_entry(_path: &Path, entry: &HistoryEntry) -> Result<(), std::io::Error> {
    add_to_blocklist(&entry.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zsh_line_extended() {
        let line = ": 1706500000:0;git status";
        let parsed = parse_zsh_line(line).expect("valid zsh extended format should parse");
        assert_eq!(parsed.command, "git status");
        assert_eq!(parsed.timestamp, Some(1706500000));
    }

    #[test]
    fn test_parse_zsh_line_plain() {
        let line = "ls -la";
        let parsed = parse_zsh_line(line).expect("plain command should parse");
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.timestamp, None);
    }

    #[test]
    fn test_parse_bash_line() {
        let line = "  cd ~/projects  ";
        assert_eq!(parse_bash_line(line), Some("cd ~/projects".to_string()));
    }

    #[test]
    fn test_parse_empty_line() {
        assert_eq!(parse_bash_line(""), None);
        assert_eq!(parse_bash_line("   "), None);
    }
}
