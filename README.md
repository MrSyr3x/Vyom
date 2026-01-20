# Vyom (व्योम) 🌌

> *Sanskrit: "The Sky", "The Void", or "The Ether"—the elemental medium through which sound travels.*

**Vyom** is a high-performance, intelligent music ecosystem for your terminal. It blends minimalist design with heavy-duty audio engineering, serving as both a tactile controller for your streaming apps (Spotify, Apple Music) and a high-fidelity MPD client with a built-in DSP engine.

![Vyom Screenshot](assets/screenshot.png?v=2)

---

## ⚡ Quick Start

```bash
# Clone and install
git clone https://github.com/MrSyr3x/Vyom.git && cd Vyom
cargo install --path . --force

# Run it!
vyom
```
*If you are in Tmux, Vyom will automatically split your window and dock itself to the side.*

---

## 🎨 Philosophy

I built **Vyom** because switching windows to skip a song is a workflow killer. I wanted my music to live where I live: **The Terminal**.

Vyom isn't just a TUI; it's the "Poweramp of the Terminal." It's designed to be transparent, blend into `neovim`, split perfectly in `tmux`, and offer a premium audio experience that rivals desktop players.

---

## ✨ Features at a Glance

| Feature | Description |
|---|---|
| **Dual Operation Modes** | **MPD Mode** (default) for local playback. **Controller Mode** (macOS only) for Spotify/Apple Music remote control. |
| **10-Band Parametric EQ** | Built-in DSP with 20+ factory presets (Bass Booster, Late Night, etc.) and **custom user presets**. |
| **Hi-Res Audio Pipeline** | Supports **24/32-bit** audio via FIFO. Dynamic sample rate detection for bit-perfect output. |
| **Synced Lyrics** | Auto-scrolling, time-synced lyrics with interactive "jump-to-time" selection. |
| **Library Browser** | Directory browser, search, playlists, and current queue management. |
| **Cava Visualizer** | Integrated spectrum analyzer for that nostalgic Hi-Fi feel. |
| **Catppuccin Themes** | Live-reloading, modern color palettes. |
| **Pixel Art Album Art** | High-fidelity album art rendered via terminal half-blocks. |
| **Tmux Aware** | Auto-detects `tmux` and docks itself as a sleek 20% sidebar. |
| **State Persistence** | Remembers your EQ settings, presets, balance, and crossfade across restarts. |

---

## 🔊 The Audio Engine

Vyom features a real-time DSP audio pipeline built from scratch.

-   **Bit-Perfect Output**: Dynamically queries MPD for the source format (sample rate, bit depth) and configures the output device accordingly. Your DAC receives the pure, untouched source.
-   **FIFO Input**: Reads Hi-Res PCM audio (16/24/32-bit) directly from a FIFO, bypassing any intermediate resampling.
-   **10-Band Biquad EQ**: A parametric equalizer with bands at 32Hz, 64Hz, 128Hz, 256Hz, 512Hz, 1kHz, 2kHz, 4kHz, 8kHz, and 16kHz. Each band is processed using precise Biquad filters.
-   **Preamp & Balance Control**: Fine-tune gain and stereo balance.
-   **Singleton Lock**: Only one Vyom instance controls audio. Other instances run in "UI-only" mode, displaying the same interface without audio contention.

---

## 🎛️ EQ Presets

Vyom ships with **20+ factory presets**, carefully tuned for different genres:

<details>
<summary>View All Presets</summary>

- Flat, Acoustic, Bass Booster, Bass Reducer, Classical, Dance, Deep
- Electronic, Hip-Hop, Jazz, Late Night, Latin, Loudness, Lounge
- Piano, Pop, R&B, Rock, Small Speakers, Spoken Word

</details>

### Custom Presets

1.  Adjust the EQ bands to your liking.
2.  Press `S` (Shift+S) to save with a name.
3.  Press `X` (Shift+X) to delete the current custom preset.

All custom presets are saved to `~/.config/vyom/state.toml` and persist across restarts.

---

## 🎮 Controls

### Global
| Key | Action |
|---|---|
| `1` / `2` / `3` / `4` | Switch views (Lyrics, Visualizer, Library, EQ) |
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous track |
| `h` / `l` | Seek backward / forward (5s) |
| `+` / `-` | Volume up / down |
| `q` | Quit |
| `?` | Show all keybindings |

### Library View (`3`)
| Key | Action |
|---|---|
| `j` / `k` | Navigate down / up |
| `h` / `l` | Go back / Enter directory or play song |
| `/` | Search library |
| `Enter` | Add song/folder to queue |
| `s` | Save current queue as playlist |
| `J` / `K` | Move item up/down in queue |

### EQ View (`4`)
| Key | Action |
|---|---|
| `←` / `→` | Select band |
| `↑` / `↓` | Adjust band gain |
| `Tab` / `Shift+Tab` | Cycle EQ presets |
| `e` | Toggle EQ on/off |
| `r` | Reset EQ to flat |
| `S` | Save current as custom preset |
| `X` | Delete current custom preset |
| `d` / `D` | Switch audio output device |

---

## 🚀 Installation

### Requirements
-   **Rust Toolchain**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
-   **MPD** (Optional): For local music library support.
-   **Cava** (Optional): For the spectrum analyzer.
-   **switchaudio-osx** (Optional, macOS): For audio device switching.

### Build & Install
```bash
# From source (MPD + EQ enabled by default)
cargo install --path . --force
```

---

## ⚙️ Configuration

Vyom stores its state and looks for configuration in your standard config home:

| File | Purpose |
|---|---|
| `~/.config/vyom/state.toml` | EQ settings, custom presets, balance, crossfade. |
| `~/.config/vyom/theme.toml` | Catppuccin theme colors (live-reloads on change). |
| `~/.config/cava/vyom_config` | Cava visualizer configuration. |

---

## 💡 Tips & Tricks

-   **Mini Player Mode**: Run `vyom --mini` for a compact view, perfect for a small corner window.
-   **Controller Mode**: Run `vyom --controller` to control Spotify or Apple Music instead of MPD.
-   **MPD Setup**: Ensure your `mpd.conf` includes a `httpd` output or a `fifo` output at `/tmp/vyom_hires.fifo` for Hi-Res audio.

---

*Made with </3 by syr3x*
