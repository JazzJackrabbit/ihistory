use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::history::HistoryEntry;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: HistoryEntry,
    pub indices: Vec<usize>,
}

pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default().ignore_case(),
        }
    }

    pub fn search(&self, entries: &[HistoryEntry], query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return entries
                .iter()
                .map(|entry| SearchResult {
                    entry: entry.clone(),
                    indices: Vec::new(),
                })
                .collect();
        }

        let query_lower = query.to_lowercase();

        let mut results: Vec<(i64, SearchResult)> = entries
            .iter()
            .filter_map(|entry| {
                let cmd_lower = entry.command.to_lowercase();
                let fuzzy_match = self.matcher.fuzzy_indices(&entry.command, query);
                let has_substring = cmd_lower.contains(&query_lower);

                if fuzzy_match.is_none() && !has_substring {
                    return None;
                }

                let (score, indices) = fuzzy_match.unwrap_or((0, Vec::new()));

                let score = if cmd_lower.starts_with(&query_lower) {
                    score + 1000
                } else {
                    score
                };

                Some((
                    score,
                    SearchResult {
                        entry: entry.clone(),
                        indices,
                    },
                ))
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));

        results.into_iter().map(|(_, r)| r).collect()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(cmd: &str) -> HistoryEntry {
        HistoryEntry {
            command: cmd.to_string(),
            timestamp: None,
            raw_line: None,
        }
    }

    fn make_entries(commands: &[&str]) -> Vec<HistoryEntry> {
        commands.iter().map(|&cmd| make_entry(cmd)).collect()
    }

    #[test]
    fn test_empty_query_returns_all() {
        let engine = SearchEngine::new();
        let entries = make_entries(&["git status", "ls -la", "cd ~"]);
        let results = engine.search(&entries, "");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_fuzzy_match() {
        let engine = SearchEngine::new();
        let entries = make_entries(&["git commit -m 'test'", "git push", "ls -la"]);
        let results = engine.search(&entries, "gco");
        assert!(!results.is_empty());
        assert!(results[0].entry.command.contains("commit"));
    }

    #[test]
    fn test_case_insensitive() {
        let engine = SearchEngine::new();
        let entries = make_entries(&["Git Status", "git push"]);
        let results = engine.search(&entries, "GIT");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_no_match() {
        let engine = SearchEngine::new();
        let entries = make_entries(&["git status", "ls -la"]);
        let results = engine.search(&entries, "xyz123");
        assert!(results.is_empty());
    }

    #[test]
    fn test_substring_match() {
        let engine = SearchEngine::new();
        let entries = make_entries(&["vim config.local.yaml", "ls -la"]);
        let results = engine.search(&entries, "local");
        assert_eq!(results.len(), 1);
        assert!(results[0].entry.command.contains("local"));
    }
}
