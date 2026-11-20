//! Bounded, Unicode-preserving names for Android direct downloads.
pub fn safe_media_stem(title: &str) -> String {
    let mut out = String::with_capacity(120);
    let mut space = false;
    for ch in title.chars() {
        if ch.is_control() || matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            continue;
        }
        if ch.is_whitespace() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            space = !out.is_empty();
            continue;
        }
        let extra = ch.len_utf8() + usize::from(space);
        if out.len() + extra > 120 { break; }
        if space { out.push(' '); space = false; }
        out.push(ch);
    }
    let trimmed = out.trim_matches(|ch: char| ch.is_whitespace() || ch == '.');
    if trimmed.is_empty() { "download".to_string() } else { trimmed.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_titles_stay_recognizable() {
        assert_eq!(safe_media_stem("Short title"), "Short title");
        assert_eq!(safe_media_stem("عنوان عربي"), "عنوان عربي");
    }
    #[test]
    fn long_unicode_has_a_byte_budget() {
        for text in ["شرح جميل ".repeat(100), "🎥".repeat(100)] {
            let name = safe_media_stem(&text);
            assert!(name.len() <= 120);
            assert!(!name.contains('\u{fffd}'));
        }
    }
    #[test]
    fn invalid_or_empty_titles_are_safe() {
        assert_eq!(safe_media_stem("../ :*?\u{0000}"), "download");
        assert_eq!(safe_media_stem("a/b\\c:d"), "a b c d");
        assert_eq!(safe_media_stem(""), "download");
    }
}
