use serde::{Deserialize, Serialize};

/// Recorded request choices, kept separate from the captions actually saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSelection {
    pub audio_requested: Vec<String>,
    pub subtitles_enabled: Option<bool>,
    pub subtitle_requested: Option<String>,
    pub automatic_subtitles: bool,
    #[serde(default)]
    pub subtitles_obtained: Vec<String>,
}

impl DownloadSelection {
    pub fn from_request(
        audio: Option<&str>,
        audios: Option<&[String]>,
        flags: Option<&[String]>,
    ) -> Self {
        let flags = flags.unwrap_or_default();
        let mut requested = audios
            .filter(|items| !items.is_empty())
            .map(<[String]>::to_vec)
            .unwrap_or_else(|| {
                audio
                    .filter(|s| !s.is_empty())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default()
            });
        let mut seen = std::collections::HashSet::new();
        requested.retain(|code| !code.is_empty() && seen.insert(code.clone()));
        Self {
            audio_requested: requested,
            subtitles_enabled: if flags.iter().any(|s| s == "--no-write-subs") {
                Some(false)
            } else if flags.iter().any(|s| s == "--write-subs") {
                Some(true)
            } else {
                None
            },
            subtitle_requested: super::subtitle_output::requested_language(Some(flags))
                .map(str::to_string),
            automatic_subtitles: !flags.iter().any(|s| s == "--no-write-auto-subs")
                && flags.iter().any(|s| s == "--write-auto-subs"),
            subtitles_obtained: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_choices_without_claiming_saved_captions() {
        let flags = vec![
            "--write-subs".into(),
            "--write-auto-subs".into(),
            "--sub-langs".into(),
            "ar-en".into(),
        ];
        let audios = vec!["ar".into(), "en".into(), "ar".into()];
        let value = DownloadSelection::from_request(None, Some(&audios), Some(&flags));
        assert_eq!(value.audio_requested, vec!["ar", "en"]);
        assert_eq!(value.subtitle_requested.as_deref(), Some("ar-en"));
        assert_eq!(value.subtitles_enabled, Some(true));
        assert!(value.automatic_subtitles);
        assert!(value.subtitles_obtained.is_empty());
    }
    #[test]
    fn retains_single_audio_with_empty_multiple_choice_array() {
        let value = DownloadSelection::from_request(Some("en"), Some(&[]), None);
        assert_eq!(value.audio_requested, vec!["en"]);
        assert_eq!(value.subtitles_enabled, None);
    }
}
