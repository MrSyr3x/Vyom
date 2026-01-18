# Vyom (व्योम) 🌌
> *Sanskrit: "The Sky", "The Void", or "The Ether"—the elemental medium through which sound travels.*

**Vyom** is a high-performance, intelligent music ecosystem for your terminal. It blends minimalist design with heavy-duty audio engineering, serving as both a controller for your streaming apps and a high-fidelity MPD client.

![Vyom Screenshot](assets/screenshot.png?v=2)

## 🎨 Why? The "Lazy & Creative" Vision 🛌💡
I built **Vyom** because switching windows to skip a song is a workflow killer. I wanted my music to live where I live: **The Terminal**. 

Vyom isn't just a TUI; it's the "Poweramp of the Terminal." It’s designed to be transparent, blend into `neovim`, split perfectly in `tmux`, and offer a premium audio experience that rivals desktop players.

## ✨ Features

- **Dual-Engine Architecture**:
    - **MPD Mode**: High-fidelity local playback with queue management, search, and directory browsing.
    - **Controller Mode**: Tactile remote control for **Spotify** and **Apple Music** (macOS native).
- **Hi-Res Audio Pipeline** 🔊:
    - Support for **24/32-bit** audio via FIFO.
    - **10-Band DSP Equalizer**: Integrated parametric EQ for fine-tuning your sound.
    - **Bit-Perfect Output**: Dynamic sample rate detection ensuring your DAC gets the pure source.
- **Visual Presence**:
    - **Catppuccin Themes**: Lush, modern color palettes with live-reloading support.
    - **"Heavenly" Pixel Art**: High-fidelity album art rendered via terminal half-blocks.
    - **Spectrum Analyzer**: Built-in visualizer (powered by Cava) for that nostalgic Hi-Fi feel.
- **Synced Lyrics** 📜: Auto-scrolling, time-synced lyrics with interactive "jump-to-time" selection.
- **Smart Layouts**:
    - **Tmux Sidebar**: Auto-detects `tmux` and docks itself as a sleek 20% sidebar.
    - **Dynamic Scaling**: Automatically shifts between Mini, Library, and Standalone views based on window size.
- **Cross-Platform**:
    - **macOS**: Native bit-perfect output and AppleScript integration.
    - **Linux**: Full compatibility with ALSA/Pulse audio support.

## 🚀 Installation

### 1. Requirements
- **Rust Toolchain**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Audio Headers**:
    - **macOS**: Built-in (CoreAudio).
    - **Linux**: `libasound2-dev` (ALSA).
- **Dependencies (Optional)**:
    - **MPD**: For local music library support.
    - **Cava**: For the spectrum analyzer.
    - **switchaudio-osx**: For device switching on Mac.

### 2. Build & Install
```bash
git clone https://github.com/MrSyr3x/Vyom.git
cd Vyom

# Install with full features (MPD + Equalizer)
cargo install --path . --features mpd,eq --force
```

## 🎮 Controls

### Global
- `1` / `2` / `3` / `4`: Switch views (Lyrics, Visualizer, Library, EQ).
- `Space`: Play/Pause.
- `n` / `p`: Next / Previous.
- `h` / `l`: Seek backward/forward (5s intervals).
- `q`: Quit.

### Library / MPD
- `/`: Trigger Global Search.
- `Enter`: Add folder/song to queue or enter directory.
- `Backspace`: Go up a directory level.
- `J` / `K`: Move items up/down in the Current Queue.
- `s`: Save current queue as a Playlist.

## ⚙️ Configuration
Vyom looks for configuration in your standard config home:
- **Theme**: `~/.config/vyom/theme.toml` (Loads automatically on change).
- **Visualizer**: `~/.config/cava/vyom_config`.

---
*Made with </3 by syr3x*
