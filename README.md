# fontes

Terminal Bible study for the King James Version with Strong's numbers, notes, search, and verse markup — all offline in a SQLite-backed TUI.

## Features

- Read the Bible with word-level Strong's tags
- Look up Strong's entries and jump to occurrences
- Full-text search across the bundled translation
- Notes anchored to words (Markdown body)
- Highlights and underlines on word ranges
- Copy verses or ranges to the clipboard with citation formatting
- Separate content database (read-only scripture) and user database (notes, annotations, reading state)

## Quick start

### Requirements

- Rust stable (2021 edition)
- Python 3.12+ (only needed to build content from source)

### Install the content bundle

Download the latest release bundle and install it into the default data directory (`~/.local/share/fontes` on Linux):

```bash
cargo install --path crates/fontes-cli --locked
fontes sync --url https://github.com/DavidHoenisch/fontes/releases/download/v1.0.0/fontes-core-kjv-strongs-1.0.0.zip
```

Or from a local zip:

```bash
fontes sync --bundle fontes-core-kjv-strongs-1.0.0.zip
```

### Run the TUI

```bash
fontes tui
fontes tui --book Jhn --chapter 3
```

## Keyboard shortcuts

The TUI also shows a summary with `?` (close with `Esc`). Keys are case-sensitive where noted.

### Reading

| Key                    | Action                                                                 |
| ---------------------- | ---------------------------------------------------------------------- |
| `←` / `→` or `h` / `l` | Previous / next word                                                   |
| `↑` / `↓` or `j` / `k` | Previous / next verse                                                  |
| `PgUp` / `PgDn`        | Jump 5 verses                                                          |
| `[` / `]`              | Previous / next chapter                                                |
| `Home` / `End`         | First / last verse in chapter                                          |
| `b`                    | Open book picker                                                       |
| `c`                    | Open chapter picker                                                    |
| `g`                    | Go to chapter or `chapter:verse`                                       |
| `/`                    | Search scripture                                                       |
| `s`                    | Strong's lookup for word under cursor                                  |
| `n`                    | New note on word under cursor                                          |
| `e`                    | Edit note on word under cursor                                         |
| `N`                    | Open notes list                                                        |
| `D`                    | Delete note on word under cursor                                       |
| `y`                    | Copy verse(s) with reference to clipboard                              |
| `V`                    | Set verse anchor — move with `j`/`k`, then `y` to copy the range       |
| `v`                    | Set word anchor — move with `h`/`l`, then `H` or `u` to mark the range |
| `H`                    | Toggle highlight on anchored word range                                |
| `u`                    | Toggle underline on anchored word range                                |
| `Esc`                  | Clear word/verse selection anchor                                      |
| `?`                    | Help overlay                                                           |
| `q`                    | Quit                                                                   |

### Go to (`g`)

| Key     | Action                                                    |
| ------- | --------------------------------------------------------- |
| Type    | Enter `3` for chapter 3, or `3:16` for chapter 3 verse 16 |
| `Enter` | Jump                                                      |
| `Esc`   | Cancel                                                    |

### Search (`/`)

| Key       | Action                                                              |
| --------- | ------------------------------------------------------------------- |
| Type      | Enter search query                                                  |
| `Enter`   | Run search (first press); jump to selected hit (when results shown) |
| `↑` / `↓` | Move through result list                                            |
| `Esc`     | Cancel and return to reading                                        |

### Book picker (`b`)

| Key       | Action                                          |
| --------- | ----------------------------------------------- |
| `↑` / `↓` | Move through book list                          |
| `Enter`   | Open selected book (chapter 1)                  |
| `/`       | Filter list by typed text                       |
| `Esc`     | Clear filter, exit filter mode, or close picker |

### Chapter picker (`c`)

| Key       | Action                                          |
| --------- | ----------------------------------------------- |
| `↑` / `↓` | Move through chapter list                       |
| `Enter`   | Open selected chapter                           |
| `/`       | Filter list by typed text                       |
| `Esc`     | Clear filter, exit filter mode, or close picker |

### Notes list (`N`)

| Key       | Action                                        |
| --------- | --------------------------------------------- |
| `↑` / `↓` | Move through notes                            |
| `Enter`   | Open selected note in editor                  |
| `/`       | Filter list by typed text                     |
| `Esc`     | Clear filter, exit filter mode, or close list |

### Strong's popup (`s`)

| Key       | Action                       |
| --------- | ---------------------------- |
| `↑` / `↓` | Move through occurrence list |
| `Enter`   | Jump to selected occurrence  |
| `Esc`     | Close popup                  |

### Note editor (`n` / `e`)

| Key      | Action                                                   |
| -------- | -------------------------------------------------------- |
| `Tab`    | Switch between title and body                            |
| Type     | Edit focused field (body supports normal text-area keys) |
| `Ctrl+s` | Save note                                                |
| `Esc`    | Cancel without saving                                    |

## CLI

| Command                                 | Description                                            |
| --------------------------------------- | ------------------------------------------------------ |
| `fontes tui`                            | Interactive terminal UI                                |
| `fontes sync --url <URL>`               | Download and install a bundle zip                      |
| `fontes sync --bundle <path>`           | Install a bundle zip from disk                         |
| `fontes sync --sqlite <path>`           | Install a `content.sqlite` directly                    |
| `fontes info`                           | Show installed bundle metadata and database paths      |
| `fontes books [--available]`            | List books (optionally only those with verse data)     |
| `fontes chapter --book Jhn --chapter 1` | Print a chapter to stdout                              |
| `fontes strong G26`                     | Look up a Strong's number                              |
| `fontes search "word of God"`           | Full-text search                                       |
| `fontes data build`                     | Build the full bundle locally (requires repo checkout) |

Set a custom data directory with `--data-dir` or `FONTES_DATA_DIR`.

## Data layout

Each data directory contains two SQLite files:

| File             | Purpose                                                                |
| ---------------- | ---------------------------------------------------------------------- |
| `content.sqlite` | Scripture, Strong's, FTS index (installed from a bundle)               |
| `user.sqlite`    | Notes, highlights, underlines, reading position (created on first run) |

Default location: `~/.local/share/fontes` (falls back to `.fontes` in the current directory if XDG paths are unavailable).

## Content bundles

Bundles are versioned zips published as [GitHub Releases](https://github.com/DavidHoenisch/fontes/releases). Each release includes:

- `fontes-core-kjv-strongs-1.0.0.zip` — `content.sqlite` + `manifest.json`
- `manifest.json` — bundle id, version, scope, and SHA-256 checksum

The current bundle (`kjv-strongs-1.0.0`) contains all 66 KJV books plus Strong's dictionary data.

### Publishing a new release

Tag push triggers the **Content bundle** workflow, which builds the bundle and attaches it to the release:

```bash
git tag v1.0.0
git push origin v1.0.0
```

You can also run the workflow manually from the Actions tab (`workflow_dispatch`) to build and verify without publishing.

### Building locally

From the repo root:

```bash
python3 tools/data/build_full.py
# or
fontes data build
```

Output lands in `data/bundles/` (gitignored). A copy is also written to `data/fixtures/` for local development.

For a smaller dev dataset (John 1–3 + Strong's):

```bash
python3 tools/data/seed_dev.py
FONTES_DATA_DIR=data/fixtures fontes tui
```

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
python3 -m unittest discover -s tools/data -p 'test_*.py'
```

CI runs Rust checks, Python unit tests, and integration tests against fresh dev fixtures on every push to `master`.

### Project layout

```
crates/
  fontes-core/   SQLite data layer (scripture, Strong's, notes, search)
  fontes-cli/    `fontes` CLI binary
  fontes-tui/    Terminal UI (ratatui)
schema/          SQL schemas for content and user databases
tools/data/      Python pipeline to fetch sources and build bundles
data/fixtures/   Dev/test databases (small subset + tracked fixtures)
```

## License

MIT
