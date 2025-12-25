# Walkthrough - Termony (formerly tmux-music)

**Termony** is a high-performance, aesthetically pleasing, and transparent music player TUI for macOS, built in Rust. It adapts intelligently to your environment.

## Key Features

### 1. Smart Window Management 🧠
*   **Tmux:** Auto-splits to the perfect size alongside your work.
*   **Standalone Mode:**
    *   **Manual Control:** Run `termony` in any terminal window.
    *   **Window Title:** Sets title to `Termony` for easy Window Manager filtering (Yabai/Amethyst).
*   **Strict Modes:**
    *   `termony`: **Mini Player Mode**. Music Only. Resizing does NOT show lyrics.
    *   `termony --lyrics`: **Full Mode**. Shows Lyrics if window is large enough.

    | Environment | Command | Behavior | Window |
    | :--- | :--- | :--- | :--- |
    | **Standalone** | `termony` | **Mini Mode** (Strict, no lyrics) | 51x33 |
    | **Standalone** | `termony --lyrics` | **Full Mode** (Lyrics enabled) | 100x80 |
    | **Tmux** | `termony` | **Full Split** (Lyrics enabled by default) | 35% Split |

    ```bash
    # Yabai Rule (Float the player)
    yabai -m rule --add title="^Termony$" manage=off
    ```

### 2. High-Performance TUI
*   **Engine:** Built with `ratatui` and `tokio`.
*   **Transparency:** Fully transparent background integration (`Color::Reset`).
*   **Responsive:** UI adapts layout based on window height.

### 3. Interactive Lyrics 🎤
*   **Manual Scroll:** Use mouse wheel to detach from sync and browse freely.
*   **Click to Jump:** Click any line to instantly seek the track to that position and re-sync.
*   **Active Highlight:** Green highlighting for the currently playing line.

### 4. Smart Catppuccin Theme ☕
*   **Palette:** Full **Catppuccin Mocha** integration.
*   **Styling:** Smooth `█▓▒░` gradient progress bar, `🎵` metadata prefix.

## Usage

### Installation
```bash
git clone https://github.com/MrSyr3x/termony.git
cd termony
cargo install --path .
```

### Running
```bash
# Auto-detects environment
termony

# Force Full Mode with Lyrics
termony --lyrics
```

### Controls
*   `Left Click`: Control playback, Seek on bar, Jump to Lyric.
*   `Scroll Wheel`: Manually scroll lyrics.
*   `Space`: Play/Pause
*   `n`: Next Track
*   `p`: Previous Track
*   `q`: Quit
