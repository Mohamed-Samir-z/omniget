//! Check requested subtitle sidecars without changing or deleting media.
use std::path::Path;

fn seconds(value: &str) -> Option<f64> {
    let mut total = 0.0;
    let fields: Vec<_> = value.split(':').collect();
    if !(2..=3).contains(&fields.len()) {
        return None;
    }
    for (index, field) in fields.iter().enumerate() {
        let value: f64 = field.replace(',', ".").parse().ok()?;
        if !value.is_finite() || value < 0.0 || (index > 0 && value >= 60.0) {
            return None;
        }
        total = total * 60.0 + value;
    }
    Some(total)
}

fn timed_cue(text: &str) -> bool {
    let lines: Vec<_> = text.lines().collect();
    lines.iter().enumerate().any(|(index, line)| {
        let Some((start, end)) = line.split_once("-->") else {
            return false;
        };
        let Some(start) = seconds(start.trim()) else {
            return false;
        };
        let Some(end) = end.split_whitespace().next().and_then(seconds) else {
            return false;
        };
        end > start
            && lines
                .iter()
                .skip(index + 1)
                .take_while(|line| !line.trim().is_empty())
                .any(|line| line.chars().any(char::is_alphanumeric))
    })
}

fn valid_file(path: &Path, ext: &str) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() == 0 || meta.len() > 16 * 1024 * 1024 {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.starts_with('<') || text.starts_with('{') {
        return false;
    }
    match ext {
        "srt" => timed_cue(text),
        "vtt" => text.starts_with("WEBVTT") && timed_cue(text),
        "ass" | "ssa" => text.lines().any(|line| {
            let Some(body) = line.strip_prefix("Dialogue:") else {
                return false;
            };
            let fields: Vec<_> = body.splitn(10, ',').collect();
            fields.len() == 10
                && seconds(fields[1].trim())
                    .zip(seconds(fields[2].trim()))
                    .is_some_and(|(a, b)| b > a)
                && fields[9].chars().any(char::is_alphanumeric)
        }),
        _ => false,
    }
}

/// Exact per-download language when provided. An off selection is checked by
/// the caller's effective download_subtitles option, never by guessing a file.
pub fn requested_language(flags: Option<&[String]>) -> Option<&str> {
    let flags = flags?;
    flags.windows(2).find_map(|pair| {
        (pair[0] == "--sub-langs" || pair[0] == "--sub-lang").then_some(pair[1].as_str())
    })
}

pub fn valid_sidecar_languages(media: &Path) -> Vec<String> {
    let (Some(parent), Some(stem)) = (media.parent(), media.file_stem().and_then(|s| s.to_str()))
    else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut languages = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if !matches!(ext, "srt" | "vtt" | "ass" | "ssa") {
            continue;
        }
        let Some(sidecar_stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(suffix) = sidecar_stem
            .strip_prefix(stem)
            .and_then(|s| s.strip_prefix('.'))
        else {
            continue;
        };
        if !suffix.is_empty() && valid_file(&path, ext) {
            languages.push(suffix.to_string());
        }
    }
    languages.sort();
    languages.dedup();
    languages
}

pub fn has_valid_sidecar(media: &Path, language: Option<&str>) -> bool {
    valid_sidecar_languages(media).iter().any(|suffix| {
        language
            .map(|lang| suffix.eq_ignore_ascii_case(lang))
            .unwrap_or(true)
    })
}

/// Settings are an ordered list of acceptable languages. Explicit picker
/// track IDs remain exact, including translated-track suffixes such as ar-en.
pub fn has_valid_selected_sidecar(
    media: &Path,
    requested: Option<&str>,
    from_settings: bool,
) -> bool {
    let Some(requested) = requested.filter(|s| !s.trim().is_empty()) else {
        return has_valid_sidecar(media, None);
    };
    valid_sidecar_languages(media).iter().any(|saved| {
        requested.split(',').map(str::trim).any(|wanted| {
            if saved.eq_ignore_ascii_case(wanted) {
                return true;
            }
            from_settings
                && !wanted.contains('-')
                && saved
                    .to_ascii_lowercase()
                    .starts_with(&format!("{}-", wanted.to_ascii_lowercase()))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "subtitle-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn absent_subtitle_never_changes_video() {
        let p = root();
        let m = p.join("video.mp4");
        std::fs::write(&m, b"kept video").unwrap();
        assert!(!has_valid_sidecar(&m, Some("ar")));
        assert_eq!(std::fs::read(&m).unwrap(), b"kept video");
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn validates_matching_language_and_cues_not_just_extension() {
        let p = root();
        let m = p.join("clip.mp4");
        let s = p.join("clip.ar.srt");
        for bad in [
            "",
            "<html>blocked</html>",
            "WEBVTT",
            "1\n00:00:03,000 --> 00:00:01,000\nخطأ\n",
            "1\n00:00:00,000 --> 00:00:01,000\n\n",
        ] {
            std::fs::write(&s, bad).unwrap();
            assert!(!has_valid_sidecar(&m, Some("ar")));
        }
        std::fs::write(&s, "1\n00:00:00,000 --> 00:00:01,200\nمرحبا\n").unwrap();
        assert!(has_valid_sidecar(&m, Some("ar")));
        assert!(!has_valid_sidecar(&m, Some("en")));
        assert!(!has_valid_sidecar(&p.join("other.mp4"), Some("ar")));
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn exact_translated_track_id_and_vtt() {
        let p = root();
        let m = p.join("clip.mp4");
        std::fs::write(
            p.join("clip.ar-en.vtt"),
            "WEBVTT\n\n00:00:00.000 --> 00:00:02.000 align:start\nمرحبا\n",
        )
        .unwrap();
        assert!(has_valid_sidecar(&m, Some("ar-en")));
        assert!(!has_valid_sidecar(&m, Some("ar")));
        assert!(has_valid_sidecar(&m, None));
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn unrelated_files_and_invalid_utf8_cannot_fulfil_selection() {
        let p = root();
        let m = p.join("clip.mp4");
        std::fs::write(p.join("clip.ar.srt"), [255, 254]).unwrap();
        std::fs::write(
            p.join("other.ar.srt"),
            "1\n00:00:00,000 --> 00:00:01,000\nHello\n",
        )
        .unwrap();
        assert!(!has_valid_sidecar(&m, Some("ar")));
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn settings_language_is_not_satisfied_by_unrelated_captions() {
        let p = root();
        let media = p.join("clip.mp4");
        std::fs::write(
            p.join("clip.en.srt"),
            "1\n00:00:00,000 --> 00:00:02,000\nEnglish\n",
        )
        .unwrap();
        assert!(!has_valid_selected_sidecar(&media, Some("ar"), true));
        assert!(has_valid_selected_sidecar(&media, Some("ar,en"), true));
        std::fs::write(
            p.join("clip.ar-en.vtt"),
            "WEBVTT\n\n00:00.000 --> 00:02.000\nمرحبا\n",
        )
        .unwrap();
        assert!(has_valid_selected_sidecar(&media, Some("ar"), true));
        assert!(!has_valid_selected_sidecar(&media, Some("ar"), false));
        assert!(has_valid_selected_sidecar(&media, Some("ar-en"), false));
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn ass_requires_timing_and_spoken_text() {
        let p = root();
        let media = p.join("clip.mp4");
        let sidecar = p.join("clip.ar.ass");
        std::fs::write(
            &sidecar,
            "[Events]\nDialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,مرحبا\n",
        )
        .unwrap();
        assert!(has_valid_sidecar(&media, Some("ar")));
        std::fs::write(
            &sidecar,
            "[Events]\nDialogue: 0,0:00:03.00,0:00:01.00,Default,,0,0,0,,مرحبا\n",
        )
        .unwrap();
        assert!(!has_valid_sidecar(&media, Some("ar")));
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn parses_explicit_language_without_changing_it() {
        let args = vec![
            "--write-auto-subs".into(),
            "--sub-langs".into(),
            "ar-en".into(),
        ];
        assert_eq!(requested_language(Some(&args)), Some("ar-en"));
        assert_eq!(requested_language(None), None);
    }
}
