# ihistory

[![CI](https://github.com/JazzJackrabbit/ihistory/actions/workflows/ci.yml/badge.svg)](https://github.com/JazzJackrabbit/ihistory/actions/workflows/ci.yml)

A fast, fuzzy shell-history search for your terminal.

`ihistory` (`ih`) is a small Rust TUI for searching shell history. Type a few characters to fuzzy-filter matches, then run, edit, or copy the selected command.

![demo](demo/demo.gif)

## Install

### Homebrew

```bash
brew trust jazzjackrabbit/tap   # newer Homebrew refuses untrusted third-party taps
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
| `Enter` | insert the selected command onto your prompt to edit |
| `Tab` | run it immediately |
| `↑` / `Ctrl-P` | move selection up |
| `↓` / `Ctrl-N` | move selection down |
| `PageUp` / `PageDown` | jump a page |
| `Ctrl-D` | hide the selected entry from results |
| `Ctrl-U` | clear the query |
| `Esc` / `Ctrl-C` | quit |

The selected command is copied to the system clipboard on exit.

Hiding an entry (`Ctrl-D`) never touches your shell's history file: the command is added to a blocklist at `~/.config/ihistory/deleted` and filtered out of future searches. Delete lines from that file to unhide.

## Building

```bash
git clone https://github.com/JazzJackrabbit/ihistory
cd ihistory
cargo build --release
```

## License

MIT © Kirill Ragozin — see [LICENSE](LICENSE).
