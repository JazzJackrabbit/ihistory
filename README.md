# ihistory

**A minimal, fast, fuzzy shell-history search for your terminal.**

`ihistory` (`ih`) is a small Rust TUI that replaces the clumsy `Ctrl-R` reverse search with an instant, fuzzy-filtered, full-screen view of your shell history. Type a few characters, see every match ranked, and run, edit, or copy the command — all without leaving the keyboard.

![demo](demo/demo.gif)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)

## Why

Shell history is one of the most-used and least-ergonomic parts of the terminal. `Ctrl-R` only shows one match at a time, has no fuzzy matching, and makes it hard to refine a search. `ihistory` loads your history into a fast fuzzy matcher and shows ranked results live as you type — so finding `docker compose up` is just `dcu`.

## Features

- **Fuzzy matching** — non-contiguous matches ranked by relevance ([`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher))
- **Full-screen TUI** — built on [`ratatui`](https://ratatui.rs) + `crossterm`, runs in an alternate screen so your prompt is untouched on exit
- **Run, insert, or copy** — execute a command, drop it onto your prompt to edit, or copy it to the system clipboard
- **Delete history entries** — prune commands you don't want lingering, right from the list
- **Shell integration** — one-line setup for `zsh` and `bash`, bindable to a key (e.g. `Ctrl-R`)
- **Fast startup** — loads up to 50k entries by default (configurable); release build is LTO'd and stripped

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

Or let `ih` detect your shell and print the right snippet:

```bash
ihistory --init        # auto-detect
```

## Usage

```bash
ih                # launch the interactive search
ih git            # launch pre-filtered to "git"
ih "git tag"      # multi-word initial query
ih -f ~/.bash_history   # search a specific history file
ih -n 100000           # raise the max entries loaded (0 = unlimited)
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

The selected command is also copied to your system clipboard on exit.

## How it works

`ih --init` adds a small shell function that calls the `ihistory` binary, which fuzzy-ranks your history in a `ratatui` TUI and hands the chosen command back to your shell to run or edit.

## Building

```bash
git clone https://github.com/JazzJackrabbit/ihistory
cd ihistory
cargo build --release
./target/release/ihistory --version
```

## License

MIT © Kirill Ragozin — see [LICENSE](LICENSE).
