//! Stable Android media variants. Keep this prefix before the title: yt-dlp may
//! trim the right-hand end of long filenames, including the video's ID.
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub struct OutputVariant<'a> {
    pub url: &'a str,
    pub quality: Option<&'a str>,
    pub mode: Option<&'a str>,
    pub audio_format: Option<&'a str>,
    pub format_id: Option<&'a str>,
    pub audio_language: Option<&'a str>,
    pub audio_languages: Option<&'a [String]>,
    pub subtitles: bool,
    pub auto_subtitles: bool,
    #[serde(skip)]
    pub flags: &'a [String],
}

fn label(value: &str, max: usize) -> String {
    value
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => Some(c),
            ',' | '+' => Some('+'),
            _ => None,
        })
        .take(max)
        .collect()
}

fn subtitle_language(flags: &[String]) -> Option<&str> {
    let mut language = None;
    for (i, flag) in flags.iter().enumerate() {
        if let Some(value) = flag.strip_prefix("--sub-langs=") {
            language = Some(value);
        } else if let Some(value) = flag.strip_prefix("--sub-lang=") {
            language = Some(value);
        } else if matches!(flag.as_str(), "--sub-langs" | "--sub-lang") {
            language = flags.get(i + 1).map(String::as_str);
        }
    }
    language
}

/// The caller supplies normalized configured + per-item flags in their actual
/// precedence order. This only changes the output template, never track choices.
pub fn filename_template(template: &str, variant: &OutputVariant<'_>) -> String {
    // Version the identity so future changes cannot silently reuse an older
    // naming policy. Transfer policy is not a different media variant.
    let media_flags: Vec<&str> = variant
        .flags
        .iter()
        .map(|s| s.trim())
        .filter(|s| {
            !s.is_empty()
                && !matches!(
                    *s,
                    "--no-overwrites"
                        | "--force-overwrites"
                        | "--overwrites"
                        | "--continue"
                        | "--no-continue"
                )
        })
        .collect();
    let identity = serde_json::to_vec(&(1_u8, variant, &media_flags))
        .expect("media variant contains only strings and booleans");
    let digest = format!("{:x}", Sha256::digest(identity));
    let audio = match variant.audio_languages.filter(|langs| !langs.is_empty()) {
        Some(langs) if langs.len() > 1 => format!("multi{}", langs.len()),
        Some(langs) => label(&langs[0], 16),
        None => label(variant.audio_language.unwrap_or("default"), 16),
    };
    let mut parts = vec![format!(
        "Audio-{}",
        if audio.is_empty() { "default" } else { &audio }
    )];
    let mut subs = variant.subtitles;
    let mut auto = variant.subtitles && variant.auto_subtitles;
    for flag in variant.flags {
        match flag.as_str() {
            "--write-subs" => subs = true,
            "--no-write-subs" => subs = false,
            "--write-auto-subs" => auto = true,
            "--no-write-auto-subs" => auto = false,
            _ => {}
        }
    }
    if subs || auto {
        let language = label(subtitle_language(variant.flags).unwrap_or("selected"), 16);
        let source = if auto && !subs { "AutoSubs" } else { "Subs" };
        parts.push(format!("{source}-{language}"));
    } else {
        parts.push("NoSubs".into());
    }
    let mode = variant.mode.unwrap_or("auto");
    if mode == "audio" {
        parts.push(format!(
            "{}-audio",
            label(variant.audio_format.unwrap_or("mp3"), 8)
        ));
    } else if mode == "mute" {
        parts.push("Silent-video".into());
    }
    if let Some(quality) = variant.quality {
        let quality = label(quality, 10);
        if !quality.is_empty() {
            parts.push(quality);
        }
    }
    // Reserve room for this prefix, a bounded video ID, language/format
    // suffixes and .part under Android's 255-byte component limit. These are
    // the standard title forms; keep all other custom template fields intact.
    let template = template
        .replace("%(title)s", "%(title,id|download).72B")
        .replace("%(title).200s", "%(title,id|download).72B")
        .replace("%(title).40s", "%(title,id|download).72B")
        .replace("%(title,id|download).120B", "%(title,id|download).72B");
    // Identity first also survives aggressive user-selected filename limits.
    // All generated characters are literals, never yt-dlp template directives.
    let prefix = format!("[v{} {}] ", &digest[..16], parts.join(" "));
    match template.rsplit_once('/') {
        Some((folder, leaf)) => format!("{folder}/{prefix}{leaf}"),
        None => format!("{prefix}{template}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant<'a>(flags: &'a [String]) -> OutputVariant<'a> {
        OutputVariant {
            url: "https://example.test/video?id=one",
            quality: Some("720p"),
            mode: Some("auto"),
            audio_format: Some("mp3"),
            format_id: None,
            audio_language: None,
            audio_languages: None,
            subtitles: false,
            auto_subtitles: false,
            flags,
        }
    }

    const TEMPLATE: &str = "%(title)s [%(id)s].%(ext)s";

    #[test]
    fn identical_choices_have_identical_resume_name() {
        let a = variant(&[]);
        assert_eq!(
            filename_template(TEMPLATE, &a),
            filename_template(TEMPLATE, &a)
        );
        assert!(filename_template(TEMPLATE, &a).contains("Audio-default NoSubs 720p"));
    }

    #[test]
    fn explicit_audio_and_subtitle_variants_are_distinct_and_readable() {
        let a = variant(&[]);
        let mut dub = variant(&[]);
        dub.audio_language = Some("ar");
        let flags = vec![
            "--write-subs".into(),
            "--no-write-auto-subs".into(),
            "--sub-langs".into(),
            "ar".into(),
        ];
        let mut sub = variant(&flags);
        sub.subtitles = true;
        let names = [
            filename_template(TEMPLATE, &a),
            filename_template(TEMPLATE, &dub),
            filename_template(TEMPLATE, &sub),
        ];
        assert_ne!(names[0], names[1]);
        assert_ne!(names[0], names[2]);
        assert_ne!(names[1], names[2]);
        assert!(names[1].contains("Audio-ar NoSubs"));
        assert!(names[2].contains("Audio-default Subs-ar"));
        // Naming has no authority to change source audio or subtitle options.
        assert_eq!(sub.audio_language, None);
        assert_eq!(sub.flags, flags);
    }

    #[test]
    fn same_language_audio_and_captions_remain_independent() {
        let flags = vec!["--write-subs".into(), "--sub-langs=en".into()];
        let mut sub = variant(&flags);
        sub.audio_language = Some("en");
        sub.subtitles = true;
        assert!(filename_template(TEMPLATE, &sub).contains("Audio-en Subs-en"));
    }

    #[test]
    fn transfer_policy_does_not_change_media_identity() {
        let flags = vec!["--no-overwrites".into(), "--continue".into()];
        assert_eq!(
            filename_template(TEMPLATE, &variant(&[])),
            filename_template(TEMPLATE, &variant(&flags))
        );
    }

    #[test]
    fn quality_format_mode_and_multiple_audio_are_distinct() {
        let baseline = filename_template(TEMPLATE, &variant(&[]));
        let mut v = variant(&[]);
        v.quality = Some("1080p");
        assert_ne!(baseline, filename_template(TEMPLATE, &v));
        v = variant(&[]);
        v.format_id = Some("398+140-21");
        assert_ne!(baseline, filename_template(TEMPLATE, &v));
        v = variant(&[]);
        v.mode = Some("audio");
        let mp3 = filename_template(TEMPLATE, &v);
        assert_ne!(baseline, mp3);
        v.audio_format = Some("m4a");
        assert_ne!(mp3, filename_template(TEMPLATE, &v));
        let languages = vec!["ar".into(), "en".into()];
        v.audio_languages = Some(&languages);
        assert!(filename_template(TEMPLATE, &v).contains("Audio-multi2"));
    }

    #[test]
    fn manual_and_auto_caption_sources_do_not_alias() {
        let manual = vec![
            "--write-subs".into(),
            "--no-write-auto-subs".into(),
            "--sub-langs".into(),
            "ar".into(),
        ];
        let auto = vec![
            "--no-write-subs".into(),
            "--write-auto-subs".into(),
            "--sub-langs".into(),
            "ar".into(),
        ];
        assert_ne!(
            filename_template(TEMPLATE, &variant(&manual)),
            filename_template(TEMPLATE, &variant(&auto))
        );
        assert!(filename_template(TEMPLATE, &variant(&auto)).contains("AutoSubs-ar"));
    }

    #[test]
    fn custom_subfolders_and_directives_are_preserved() {
        let name = filename_template("%(uploader)s/Custom.%(ext)s", &variant(&[]));
        assert!(name.starts_with("%(uploader)s/[v"));
        assert!(name.ends_with("Custom.%(ext)s"));
        let hostile = vec!["--sub-langs".into(), "ar/../../%(title)s\n".into()];
        let mut v = variant(&hostile);
        v.subtitles = true;
        let name = filename_template(TEMPLATE, &v);
        assert_eq!(name.matches("%(title,id|download).72B").count(), 1);
        assert!(!name.contains("../"));
        assert!(!name.contains('\n'));
    }

    #[test]
    fn title_trimming_cannot_erase_variant_or_source_identity() {
        let a = filename_template(TEMPLATE, &variant(&[]));
        let mut b = variant(&[]);
        b.url = "https://example.test/video?id=two";
        let b = filename_template(TEMPLATE, &b);
        assert_ne!(&a[..18], &b[..18]);
        // Even a 20-byte limit retains the complete 64-bit identity.
        assert_ne!(&a[..20], &b[..20]);
        assert!(a.starts_with("[v"));
    }

    #[test]
    fn existing_file_is_never_opened_or_renamed_by_naming() {
        let old = "Existing title [id].mp4";
        let name = filename_template(old, &variant(&[]));
        assert_ne!(name, old);
        assert!(name.ends_with(old));
    }
}
