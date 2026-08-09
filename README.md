# ihistory

[![CI](https://github.com/JazzJackrabbit/ihistory/actions/workflows/ci.yml/badge.svg)](https://github.com/JazzJackrabbit/ihistory/actions/workflows/ci.yml)

A replacement for your shell's Ctrl+R.

The built-in reverse search is exact-substring, one match at a time, blind. `ihistory` (`ih`) is a small Rust TUI that puts a ranked, fuzzy-filtered list of your history on that same keybinding — with previews for long commands, timestamps, and the ability to hide entries from results. One binary, one line of shell setup.

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

The integration binds `Ctrl+R` and defines the `ih` command. To keep your shell's own `Ctrl+R`, export `IHISTORY_NO_BINDKEY=1` before the eval line.

## Usage

Press `Ctrl+R` at your prompt. Type to filter, pick a command, then `Enter` to put it back on your prompt for editing or `Tab` to run it straight away. The selection is copied to the clipboard either way.

The `ih` command opens the same search and takes arguments:

```bash
ih                      # interactive search
ih git                  # pre-filtered to "git"
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

Hiding an entry (`Ctrl-D`) never touches your shell's history file: the command is added to a blocklist at `~/.config/ihistory/deleted` and filtered out of future searches. Delete lines from that file to unhide.

## Building

```bash
git clone https://github.com/JazzJackrabbit/ihistory
cd ihistory
cargo build --release
```

## License

MIT © Kirill Ragozin — see [LICENSE](LICENSE).
