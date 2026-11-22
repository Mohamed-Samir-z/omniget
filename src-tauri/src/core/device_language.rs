/// Only called for a genuinely absent settings record. Existing choices,
/// including explicit English, are never replaced on later launches.
pub fn initial_device_language(saved: Option<&str>, device: &str) -> String {
    if let Some(saved) = saved {
        return saved.to_string();
    }
    for language in device.split(',') {
        let tag = language.trim().replace('_', "-").to_ascii_lowercase();
        if ["zh-tw", "zh-hk", "zh-mo", "zh-hant"]
            .iter()
            .any(|v| tag.starts_with(v))
        {
            return "zh-TW".into();
        }
        let primary = tag.split('-').next().unwrap_or("");
        if [
            "ar", "en", "ru", "el", "pt", "zh", "ja", "it", "fr", "es", "fa", "lo",
        ]
        .contains(&primary)
        {
            return primary.into();
        }
    }
    "en".into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_device_languages_and_variants() {
        for (device, want) in [
            ("ar-EG", "ar"),
            ("en-US", "en"),
            ("zh-Hant-TW", "zh-TW"),
            ("zh-CN", "zh"),
            ("fr_FR", "fr"),
            ("de-DE,ar-EG", "ar"),
        ] {
            assert_eq!(initial_device_language(None, device), want);
        }
    }
    #[test]
    fn unsupported_uses_english() {
        assert_eq!(initial_device_language(None, "de-DE"), "en");
        assert_eq!(initial_device_language(None, ""), "en");
    }
    #[test]
    fn explicit_existing_language_always_survives() {
        for saved in ["en", "ar", "fr"] {
            assert_eq!(initial_device_language(Some(saved), "ar-EG"), saved);
        }
    }
}
