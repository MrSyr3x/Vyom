/// Safely truncate string to max characters, appending "…" if truncated 🛡️
pub fn truncate(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_width {
        chars
            .into_iter()
            .take(max_width.saturating_sub(1))
            .collect::<String>()
            + "…"
    } else {
        s.to_string()
    }
}
