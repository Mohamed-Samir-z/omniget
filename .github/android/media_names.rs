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

/// Byte-bounded yt-dlp default, including the caller's explicit None branch.
pub fn download_filename_template(template: Option<&str>) -> String {
    template.unwrap_or("%(title).180s [%(id)s].%(ext)s").replace("%(title).200s", "%(title,id|download).120B").replace("%(title).180s", "%(title,id|download).120B").replace("%(title).40s", "%(title,id|download).120B").replace("%(title)s", "%(title,id|download).120B").replace("%(id)s", "%(id).32B")
}
#[cfg(test)]
mod filename_template_tests {
    use super::*;
    #[test] fn none_template_is_byte_bounded_for_arabic_and_emoji() {
        let template=download_filename_template(None);
        assert_eq!(template,"%(title,id|download).120B [%(id).32B].%(ext)s");
        for title in ["عنوان عربي طويل ".repeat(80),"🎥✨".repeat(100)] {
            assert!(title.len()>255);let bounded=safe_media_stem(&title);
            let filename=format!("{bounded} [12345678901234567890123456789012].f999.mp4.part");
            assert!(filename.len()<255);assert!(!filename.contains('\u{fffd}'));
        }
    }
}
