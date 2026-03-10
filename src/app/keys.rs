use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    // Global
    pub quit: String,
    pub play_pause: String,
    pub next_track: String,
    pub prev_track: String,
    pub volume_up: String,
    pub volume_down: String,
    pub toggle_keyhints: String,
    pub toggle_audio_info: String,
    pub search_global: String,

    // View Switching
    pub view_lyrics: String,
    pub view_visualizer: String,
    pub view_library: String,
    pub view_eq: String,
    
    // Artwork
    pub cycle_art: String,

    // Seek
    pub seek_forward: String,
    pub seek_backward: String,

    // Navigation (Shared)
    pub nav_up: String,
    pub nav_up_alt: String,
    pub nav_down: String,
    pub nav_down_alt: String,
    pub nav_left: String,
    pub nav_left_alt: String,
    pub nav_right: String,
    pub nav_right_alt: String,

    // Library
    pub enter_dir: String,
    pub back_dir: String,
    pub back_dir_alt: String, // Esc/Backspace match
    pub add_to_queue: String,
    pub save_playlist: String,
    pub rename_playlist: String,
    pub delete_item: String,
    pub edit_tags: String,
    pub move_down: String,
    pub move_up: String,
    pub tab_next: String,
    pub tab_prev: String,

    // Lyrics
    pub seek_to_line: String,

    // EQ
    pub band_next: String,
    pub band_prev: String,
    pub gain_up: String,
    pub gain_down: String,
    pub toggle_eq: String,
    pub reset_eq: String,
    pub reset_levels: String,
    pub next_preset: String, // uses tab_next
    pub prev_preset: String, // uses tab_prev
    pub save_preset: String,
    pub delete_preset: String,
    pub preamp_up: String,
    pub preamp_down: String,
    pub balance_right: String,
    pub balance_left: String,
    pub crossfade: String,
    pub replay_gain: String,
    pub device_next: String,
    pub device_prev: String,

    // MPD
    pub shuffle: String,
    pub repeat: String,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            play_pause: "Space".to_string(),
            next_track: "n".to_string(),
            prev_track: "p".to_string(),
            volume_up: "+".to_string(),
            volume_down: "-".to_string(),
            toggle_keyhints: "?".to_string(),
            toggle_audio_info: "i".to_string(),
            search_global: "/".to_string(),

            view_lyrics: "1".to_string(),
            view_visualizer: "2".to_string(),
            view_library: "3".to_string(),
            view_eq: "4".to_string(),
            
            cycle_art: "A".to_string(),

            seek_forward: "l".to_string(),
            seek_backward: "h".to_string(),

            nav_up: "k".to_string(),
            nav_up_alt: "Up".to_string(),
            nav_down: "j".to_string(),
            nav_down_alt: "Down".to_string(),
            nav_left: "h".to_string(),
            nav_left_alt: "Left".to_string(),
            nav_right: "l".to_string(),
            nav_right_alt: "Right".to_string(),

            enter_dir: "Enter".to_string(),
            back_dir: "Backspace".to_string(),
            back_dir_alt: "Esc".to_string(),
            add_to_queue: "a".to_string(),
            save_playlist: "s".to_string(),
            rename_playlist: "r".to_string(),
            delete_item: "d".to_string(),
            edit_tags: "t".to_string(),
            move_down: "J".to_string(),
            move_up: "K".to_string(),
            tab_next: "Tab".to_string(),
            tab_prev: "BackTab".to_string(),

            seek_to_line: "Enter".to_string(),

            band_next: "l".to_string(),
            band_prev: "h".to_string(),
            gain_up: "k".to_string(),
            gain_down: "j".to_string(),
            toggle_eq: "e".to_string(),
            reset_eq: "r".to_string(),
            reset_levels: "0".to_string(),
            next_preset: "Tab".to_string(),
            prev_preset: "BackTab".to_string(),
            save_preset: "S".to_string(),
            delete_preset: "X".to_string(),
            preamp_up: "g".to_string(),
            preamp_down: "G".to_string(),
            balance_right: "b".to_string(),
            balance_left: "B".to_string(),
            crossfade: "c".to_string(),
            replay_gain: "R".to_string(),
            device_next: "d".to_string(),
            device_prev: "D".to_string(),

            shuffle: "z".to_string(),
            repeat: "x".to_string(),
        }
    }
}

impl KeyConfig {
    pub fn matches(&self, event: KeyEvent, key_str: &str) -> bool {
        match key_str {
            "Space" => event.code == KeyCode::Char(' '),
            "Enter" => event.code == KeyCode::Enter,
            "Backspace" => event.code == KeyCode::Backspace,
            "Esc" => event.code == KeyCode::Esc,
            "Tab" => event.code == KeyCode::Tab,
            "BackTab" => event.code == KeyCode::BackTab,
            "Up" => event.code == KeyCode::Up,
            "Down" => event.code == KeyCode::Down,
            "Left" => event.code == KeyCode::Left,
            "Right" => event.code == KeyCode::Right,
            s if s.len() == 1 => {
                if let Some(ch) = s.chars().next() {
                    // Check for shift modifier if char is uppercase
                    if ch.is_uppercase() {
                        event.code == KeyCode::Char(ch)
                            || (event.code == KeyCode::Char(ch.to_ascii_lowercase())
                                && event.modifiers.contains(KeyModifiers::SHIFT))
                    } else {
                        event.code == KeyCode::Char(ch)
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // Helper for UI display
    pub fn display(&self, key_str: &str) -> String {
        match key_str {
            "Space" => "Space".to_string(),
            "Up" => "↑".to_string(),
            "Down" => "↓".to_string(),
            "Left" => "←".to_string(),
            "Right" => "→".to_string(),
            "BackTab" => "S-Tab".to_string(), // Shift+Tab
            "Backspace" => "Bksp".to_string(),
            _ => key_str.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_matches_space() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Char(' ')), &cfg.play_pause));
    }

    #[test]
    fn test_matches_enter() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Enter), &cfg.enter_dir));
    }

    #[test]
    fn test_matches_backspace() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Backspace), &cfg.back_dir));
    }

    #[test]
    fn test_matches_esc() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Esc), &cfg.back_dir_alt));
    }

    #[test]
    fn test_matches_tab() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Tab), &cfg.tab_next));
    }

    #[test]
    fn test_matches_backtab() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::BackTab), &cfg.tab_prev));
    }

    #[test]
    fn test_matches_arrows() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Up), &cfg.nav_up_alt));
        assert!(cfg.matches(key(KeyCode::Down), &cfg.nav_down_alt));
        assert!(cfg.matches(key(KeyCode::Left), &cfg.nav_left_alt));
        assert!(cfg.matches(key(KeyCode::Right), &cfg.nav_right_alt));
    }

    #[test]
    fn test_matches_lowercase_char() {
        let cfg = KeyConfig::default();
        assert!(cfg.matches(key(KeyCode::Char('q')), &cfg.quit));
        assert!(cfg.matches(key(KeyCode::Char('n')), &cfg.next_track));
        assert!(cfg.matches(key(KeyCode::Char('p')), &cfg.prev_track));
    }

    #[test]
    fn test_matches_uppercase_with_shift() {
        let cfg = KeyConfig::default();
        // Shift+a should match "A"
        assert!(cfg.matches(key_shift(KeyCode::Char('a')), &cfg.cycle_art));
        // Direct uppercase char should also match
        assert!(cfg.matches(key(KeyCode::Char('A')), &cfg.cycle_art));
    }

    #[test]
    fn test_no_false_positive() {
        let cfg = KeyConfig::default();
        // 'q' should not match play_pause ("Space")
        assert!(!cfg.matches(key(KeyCode::Char('q')), &cfg.play_pause));
        // Enter should not match quit ("q")
        assert!(!cfg.matches(key(KeyCode::Enter), &cfg.quit));
    }

    #[test]
    fn test_display_special_keys() {
        let cfg = KeyConfig::default();
        assert_eq!(cfg.display("Space"), "Space");
        assert_eq!(cfg.display("Up"), "↑");
        assert_eq!(cfg.display("Down"), "↓");
        assert_eq!(cfg.display("Left"), "←");
        assert_eq!(cfg.display("Right"), "→");
        assert_eq!(cfg.display("BackTab"), "S-Tab");
        assert_eq!(cfg.display("Backspace"), "Bksp");
        assert_eq!(cfg.display("q"), "q");
    }
}
