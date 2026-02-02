# ihistory

A minimal, fast, fuzzy shell history search tool.

## Installation

### Homebrew (recommended)

```bash
brew install jazzjackrabbit/tap/ihistory
```

Then add to your `~/.zshrc` or `~/.bashrc`:

```bash
source "$(brew --prefix)/share/ihistory/ihistory.sh"
```

### From crates.io

```bash
cargo install ihistory
ln -s ~/.cargo/bin/ihistory ~/.cargo/bin/ih  # optional
```

### From source

```bash
git clone https://github.com/jazzjackrabbit/ihistory
cd ihistory
cargo install --path .
ln -s ~/.cargo/bin/ihistory ~/.cargo/bin/ih  # optional
```

## Usage

```bash
ih           # launch interactive search
ih git       # start with "git" as query
ih --help    # see all options
```

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `Ctrl+p` | Move up |
| `↓` / `Ctrl+n` | Move down |
| `PageUp` / `PageDown` | Move 20 items |
| `Enter` | Select (copy to buffer) |
| `Tab` | Select and execute |
| `Ctrl+D` | Delete entry from history |
| `Ctrl+U` | Clear search |
| `Esc` / `Ctrl+C` | Cancel |

## How it works

1. Reads your shell history file (`~/.zsh_history` or `~/.bash_history`)
2. Parses both bash (simple) and zsh (extended timestamp) formats
3. Deduplicates entries, keeping most recent first
4. Provides fuzzy search using the skim algorithm
5. Outputs selected command to stdout for shell integration

## License

MIT
