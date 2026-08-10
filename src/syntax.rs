//! A small shell tokenizer for display coloring. One pass over chars, no
//! dependencies; it only has to be right enough to read, not to parse shell.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The command word (first word, and first word after |, &&, ;, etc.)
    Command,
    /// -f, --long-flag
    Flag,
    /// 'single' and "double" quoted spans, quotes included
    Str,
    /// $VAR, ${...}
    Var,
    /// | & ; > < ( )
    Operator,
    Plain,
}

/// Words that prefix another command rather than being the command itself.
const COMMAND_PREFIXES: [&str; 5] = ["sudo", "env", "command", "exec", "nohup"];

/// Builtins and keywords that never appear in $PATH but are commands.
const BUILTINS: [&str; 32] = [
    "cd", "echo", "export", "source", "alias", "unalias", "set", "unset", "eval", "exec", "exit",
    "fg", "bg", "jobs", "history", "type", "print", "pushd", "popd", "dirs", "read", "test", "let",
    "local", "declare", "typeset", "if", "for", "while", "time", "unsetopt", "command",
];

/// The names a bare word can resolve to as a command: everything in $PATH
/// plus builtins. Built once at startup — a directory listing per PATH entry,
/// no per-file stat — so classification is a hash lookup per word.
pub struct CommandIndex(HashSet<String>);

impl CommandIndex {
    pub fn from_path() -> Self {
        let mut names: HashSet<String> = BUILTINS.iter().map(|s| s.to_string()).collect();
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Ok(name) = entry.file_name().into_string() {
                            names.insert(name);
                        }
                    }
                }
            }
        }
        Self(names)
    }

    #[cfg(test)]
    pub fn from_names(names: &[&str]) -> Self {
        Self(names.iter().map(|s| s.to_string()).collect())
    }

    /// A word is invokable if it is a known name, or an explicit path.
    fn is_command(&self, word: &str) -> bool {
        word.contains('/') || word.starts_with('~') || self.0.contains(word)
    }
}

/// The preference is a marker file, like the hide blocklist: present means
/// highlighting is off. Highlighting defaults to on.
fn pref_path() -> Option<std::path::PathBuf> {
    let dir = dirs::config_dir()?.join("ihistory");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("nohighlight"))
}

pub fn enabled() -> bool {
    pref_path().map(|p| !p.exists()).unwrap_or(true)
}

pub fn set_enabled(on: bool) {
    if let Some(path) = pref_path() {
        if on {
            let _ = std::fs::remove_file(path);
        } else {
            let _ = std::fs::write(path, b"");
        }
    }
}

/// Classifies every char of `input`. The result is aligned with
/// `input.chars()` indices — the same indexing the fuzzy matcher uses.
/// With an index, a word in command position is only classed Command when it
/// actually resolves to one — a typo'd or pasted first word stays Plain.
pub fn classify_with(input: &str, index: Option<&CommandIndex>) -> Vec<Class> {
    let chars: Vec<char> = input.chars().collect();
    let mut classes = vec![Class::Plain; chars.len()];
    let mut i = 0;
    // The next bare word is the command (start of line / after an operator).
    let mut expect_command = true;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Quoted string: color the whole span, respecting \" inside doubles.
        if c == '\'' || c == '"' {
            let quote = c;
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && quote == '"' {
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            classes[start..i.min(chars.len())].fill(Class::Str);
            continue;
        }

        // Variable: $NAME or ${...}
        if c == '$' && i + 1 < chars.len() {
            let start = i;
            i += 1;
            if chars[i] == '{' {
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                i = (i + 1).min(chars.len());
            } else {
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
            }
            if i > start + 1 {
                classes[start..i].fill(Class::Var);
                continue;
            }
            i = start; // lone '$': fall through as a plain word char
        }

        // Operators separate pipeline stages; the word after one is a command.
        if matches!(c, '|' | '&' | ';' | '>' | '<' | '(' | ')') {
            classes[i] = Class::Operator;
            if matches!(c, '|' | '&' | ';' | '(') {
                expect_command = true;
            }
            i += 1;
            continue;
        }

        // A word: consume to the next whitespace/quote/operator boundary.
        let start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && !matches!(
                chars[i],
                '\'' | '"' | '|' | '&' | ';' | '>' | '<' | '(' | ')' | '$'
            )
        {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();

        let known = index.map(|ix| ix.is_command(&word)).unwrap_or(true);
        let class = if word.starts_with('-') && word.len() > 1 {
            Class::Flag
        } else if expect_command && known {
            Class::Command
        } else {
            Class::Plain
        };
        classes[start..i].fill(class);

        if expect_command && !word.starts_with('-') {
            // sudo/env/etc. defer the command role to the next word.
            expect_command = COMMAND_PREFIXES.contains(&word.as_str());
        }

        // A char no branch consumed (a lone '$', "$(" substitution) would
        // otherwise loop here forever — the freeze is worse than an
        // unclassified char, so always make progress.
        if i == start {
            i += 1;
        }
    }

    classes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes_for(input: &str) -> Vec<(char, Class)> {
        input.chars().zip(classify_with(input, None)).collect()
    }

    fn class_of_word(input: &str, word: &str) -> Class {
        let start = input.find(word).expect("word present");
        let char_start = input[..start].chars().count();
        classify_with(input, None)[char_start]
    }

    #[test]
    fn dollar_forms_terminate() {
        // "$(" and a trailing "$" once looped forever mid-render.
        for cmd in [
            "echo $(date)",
            "echo $",
            "watch -n1 $(cat /tmp/x) | grep y",
            "$",
        ] {
            let classes = classify_with(cmd, None);
            assert_eq!(classes.len(), cmd.chars().count());
        }
    }

    #[test]
    fn index_gates_command_highlighting() {
        let ix = CommandIndex::from_names(&["git"]);
        let classes = classify_with("gti status", Some(&ix));
        assert_eq!(classes[0], Class::Plain, "typo must not read as a command");
        let classes = classify_with("git status", Some(&ix));
        assert_eq!(classes[0], Class::Command);
        // Explicit paths are invokable regardless of the index.
        let classes = classify_with("./run.sh --fast", Some(&ix));
        assert_eq!(classes[0], Class::Command);
        // An unknown word does not hand command position to the next word.
        let classes = classify_with("gti status", Some(&ix));
        let s_pos = "gti ".chars().count();
        assert_eq!(classes[s_pos], Class::Plain);
    }

    #[test]
    fn first_word_is_the_command() {
        assert_eq!(class_of_word("git status", "git"), Class::Command);
        assert_eq!(class_of_word("git status", "status"), Class::Plain);
    }

    #[test]
    fn flags_and_strings() {
        let cmd = r#"git commit -m "fix login bug""#;
        assert_eq!(class_of_word(cmd, "-m"), Class::Flag);
        assert_eq!(class_of_word(cmd, "\"fix"), Class::Str);
    }

    #[test]
    fn command_after_pipe_and_sudo() {
        assert_eq!(
            class_of_word("docker ps | grep api", "grep"),
            Class::Command
        );
        assert_eq!(
            class_of_word("sudo systemctl restart nginx", "systemctl"),
            Class::Command
        );
        assert_eq!(
            class_of_word("sudo systemctl restart nginx", "restart"),
            Class::Plain
        );
    }

    #[test]
    fn variables() {
        assert_eq!(class_of_word("echo $HOME/bin", "$HOME"), Class::Var);
        assert_eq!(class_of_word("echo ${PATH}x", "${PATH}"), Class::Var);
    }

    #[test]
    fn alignment_survives_unicode() {
        // Classification is char-indexed, like the matcher's indices.
        let pairs = classes_for("gít 'ünïcöde'");
        assert_eq!(pairs[0], ('g', Class::Command));
        assert_eq!(pairs[4], ('\'', Class::Str));
        assert_eq!(pairs.last().unwrap().1, Class::Str);
    }
}
