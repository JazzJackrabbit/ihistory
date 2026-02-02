# ihistory

A fast, fuzzy shell history search.

![demo](demo/demo.gif)

## Install

```bash
brew install jazzjackrabbit/tap/ihistory
```

Then add to your `~/.zshrc` or `~/.bashrc`:

```bash
source "$(brew --prefix)/share/ihistory/ihistory.sh"
```

## Usage

```bash
ih           # search history
ih git       # search with initial query
```

| Key | Action |
|-----|--------|
| `Enter` | Select |
| `Tab` | Select and run |
| `Ctrl+D` | Delete entry |
| `Esc` | Cancel |

## License

MIT
