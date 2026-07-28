# ♪ Playful

A lightweight, keyboard-driven terminal music player for the [Kitty terminal](https://sw.kovidgoyal.net/kitty/).

Built with Rust, ratatui, and mpv as the audio backend. No bloated GUIs — just your terminal and your music.

## Features

- **Album art** — embedded cover art displayed via Kitty's graphics protocol (square, compact, no distortion)
- **File details** — shows format, bitrate, and year for the selected track
- **Keyboard-first** — full navigation without touching the mouse
- **Rainbow header** — because why not
- **Configurable music folder** — set it on first launch or change it anytime
- **Library scanner** — recursively scans your music folder; supports MP3, FLAC, M4A, OGG, WAV, WMA, AAC, Opus
- **Progress bar** — visual playback progress with elapsed/total time
- **Volume control** — fine-tune with +/- keys

## Dependencies

- **mpv** — audio playback (must be installed and available in `$PATH`)
- **Kitty terminal** — album art uses the Kitty graphics protocol

## Installation

### 1. Install mpv

```bash
sudo apt install mpv
```

### 2. Install playful

**Option A — Download the pre-built binary (recommended):**

Grab the latest `playful` binary from the [Releases page](https://github.com/fernandorodriguezrada/playful/releases), then:

```bash
chmod +x playful
sudo mv playful /usr/local/bin/
```

**Option B — Build from source with Cargo:**

```bash
cargo install --git https://github.com/fernandorodriguezrada/playful
```

## Usage

Just run:

```bash
playful
```

On first launch, you'll be prompted to enter your music folder path. This is saved to `~/.config/playful/config.json`.

### Keybindings

| Key | Action |
|---|---|
| `↑` / `↓` or `j` / `k` | Navigate track list |
| `PgUp` / `PgDn` | Scroll 10 tracks |
| `Home` / `End` | Jump to first/last track |
| `Enter` | Play selected track / Pause (if currently playing) |
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous track |
| `←` / `→` | Seek backward / forward 5s |
| `,` / `.` | Seek backward / forward 5s |
| `s` | Stop |
| `+` / `-` | Volume up / down |
| `Alt+;` | Open command palette |
| `c` | Change music folder |
| `r` | Refresh library |
| `q` | Quit |

## Configuration

Config is stored at `~/.config/playful/config.json`. Currently only the music folder path is persisted.

## Architecture

- **`app.rs`** — main state machine, event loop, key handling, cover art management
- **`ui.rs`** — all TUI rendering using ratatui
- **`library.rs`** — music file scanning and metadata extraction via lofty
- **`player.rs`** — mpv subprocess management via JSON IPC
- **`config.rs`** — XDG-compliant config save/load

## License

MIT