# ihistory

A fast, fuzzy shell-history search for your terminal.

`ihistory` (`ih`) is a small Rust TUI for searching shell history. Type a few characters to fuzzy-filter matches, then run, edit, or copy the selected command.

![demo](demo/demo.gif)

## Features

- Fuzzy matching ranked by relevance ([`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher))
- Full-screen TUI built on [`ratatui`](https://ratatui.rs) and `crossterm`, running in an alternate screen
- Run a command, insert it onto your prompt to edit, or copy it to the clipboard
- Delete history entries directly from the list
- One-line shell integration for `zsh` and `bash`
- Loads up to 50k entries by default (configurable)

## Install

### Homebrew

```bash
brew install jazzjackrabbit/tap/ihistory
```

### From source

```bash
cargo install --git https://github.com/JazzJackrabbit/ihistory
```

## Setup

Add shell integration to your config:

```bash
# zsh — in ~/.zshrc
eval "$(ihistory --init zsh)"

# bash — in ~/.bashrc
eval "$(ihistory --init bash)"
```

Or let `ih` auto-detect your shell:

```bash
ihistory --init
```

## Usage

```bash
ih                      # launch the interactive search
ih git                  # launch pre-filtered to "git"
ih "git tag"            # multi-word initial query
ih -f ~/.bash_history   # search a specific history file
ih -n 100000            # raise the max entries loaded (0 = unlimited)
```

### Keybindings

| Key | Action |
| --- | --- |
| _type_ | filter history fuzzily |
| `Enter` | run the selected command |
| `Tab` | insert it onto your prompt to edit |
| `↑` / `Ctrl-P` | move selection up |
| `↓` / `Ctrl-N` | move selection down |
| `PageUp` / `PageDown` | jump a page |
| `Ctrl-D` | delete the selected entry from history |
| `Ctrl-U` | clear the query |
| `Esc` / `Ctrl-C` | quit |

The selected command is copied to the system clipboard on exit.

## Building

```bash
git clone https://github.com/JazzJackrabbit/ihistory
cd ihistory
cargo build --release
```

## License

MIT © Kirill Ragozin — see [LICENSE](LICENSE).
