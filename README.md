# Vyom (व्योम) 🌌
> *Sanskrit: "The Sky", "The Void", or "The Ether"—the elemental medium through which sound travels.*

**Vyom** is a high-performance, intelligent music player for the terminal.

![Vyom Screenshot](assets/screenshot.png?v=2)

## Why? The "Lazy & Creative" Origin Story 🛌💡

I built **Vyom** because I was tired. 

I constantly forget which Desktop Space my Spotify or Apple Music window is hiding on. I was exhausted from spamming `Option + 1`, `Option + 2`, `Option + 3`... just to find the window to skip a track or see the lyrics.

I wanted something that lives where I live: **The Terminal**. 

I didn't just want a controller; I wanted an experience. Something transparent that blends into my `neovim` workflow, something that splits perfectly in `tmux`, and something that looks "heavenly" with pixel-perfect artwork.

I am lazy enough to automate the annoyance, and creative enough to make it beautiful. Thus, **Vyom** was born.

## Features ✨

*   **Intelligent Layouts:** 
    *   **Tmux Mode:** Auto-splits to a perfect 35% sidebar.
    *   **Standalone Mode:** Switches to a massive "Apple Music" style layout when you make the window huge (>120 cols).
    *   **Mini Mode:** Shrinks down to just the essentials when space is tight.
*   **"Heavenly" Pixel Art:** High-quality, aspect-ratio corrected album art rendered directly in the terminal using half-blocks (`▀`).
*   **Synced Lyrics:** Live, scrolling lyrics that you can interact with (click lines to seek!).
*   **Transparent:** Fully transparent UI that respects your terminal's background.

## What You Need 🛠️

To run **Vyom**, you need:

1.  **macOS**: Works via AppleScript to talk to Spotify/Apple Music.
2.  **Spotify** (Desktop App) running.
3.  **Rust**: To build it.

## Installation 🚀

```bash
git clone https://github.com/MrSyr3x/vyom.git
cd vyom
cargo install --path .
```

## How to Use 🎮

**The Mini Player (Minimalist):**
```bash
vyom
```
*Perfect for a small corner window.*

**The Full Experience (Lyrics + Big Art):**
```bash
vyom --lyrics
```
*If you are in Tmux, this will automatically split your window and dock Vyom to the side.*

**Controls:**
*   `Space`: Play/Pause
*   `n` / `p`: Next / Previous Track
*   `Mouse`: Click progress bar to seek, click lyric lines to jump.
*   `q`: Quit

---
*Made with </3 by syr3x*
