//! Per-download clipping: validate ranges, isolate intermediates and publish a
//! timestamp-normalized file under a collision-safe title/range name.
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub struct ClipRange {
    pub start: f64,
    pub end: Option<f64>,
}

fn seconds(text: &str) -> Option<f64> {
    let fields: Vec<_> = text.split(':').collect();
    if fields.is_empty() || fields.len() > 3 || fields.iter().any(|v| v.is_empty()) {
        return None;
    }
    let mut total = 0.0;
    for (i, field) in fields.iter().enumerate() {
        if !field.chars().all(|c| c.is_ascii_digit() || c == '.')
            || (i + 1 != fields.len() && field.contains('.'))
        {
            return None;
        }
        let n: f64 = field.parse().ok()?;
        if !n.is_finite() || n < 0.0 || (i > 0 && n >= 60.0) {
            return None;
        }
        total = total * 60.0 + n;
    }
    (total <= 31_536_000.0).then_some(total)
}

pub fn display_time(value: f64) -> String {
    let ms = (value * 1000.0).round() as u64;
    let s = ms / 1000;
    let mut text = if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    };
    if !ms.is_multiple_of(1000) {
        text.push_str(format!(".{:03}", ms % 1000).trim_end_matches('0'));
    }
    text
}

impl ClipRange {
    pub fn parse(text: &str) -> Result<Self, String> {
        let (a, b) = text.split_once('-').ok_or("Enter a start and end time")?;
        let start = seconds(a).ok_or("Invalid clip start time")?;
        let end = if b == "inf" {
            None
        } else {
            Some(seconds(b).ok_or("Invalid clip end time")?)
        };
        if end.is_some_and(|end| end <= start) {
            return Err("Clip end must be after the start".into());
        }
        Ok(Self { start, end })
    }

    pub fn from_args(args: Option<&[String]>) -> Option<Self> {
        args?.windows(2).find_map(|pair| {
            (pair[0] == "--download-sections")
                .then(|| Self::parse(pair[1].strip_prefix('*')?).ok())
                .flatten()
        })
    }

    pub fn expected_duration(&self, source_duration: Option<f64>) -> anyhow::Result<Option<f64>> {
        let source = source_duration.filter(|d| d.is_finite() && *d > 0.0);
        if source.is_some_and(|duration| self.start >= duration) {
            bail!("Clip start is past the end of this video");
        }
        let end = match (self.end, source) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        Ok(end.map(|end| end - self.start))
    }

    pub fn clamped_end(&self, source_duration: Option<f64>) -> Option<f64> {
        let duration = source_duration.filter(|d| d.is_finite() && *d > self.start)?;
        self.end.filter(|end| *end > duration).map(|_| duration)
    }

    fn suffix(&self) -> String {
        format!(
            " (from {} to {})",
            display_time(self.start),
            self.end.map(display_time).unwrap_or_else(|| "end".into())
        )
    }
}

// Section downloads can end slightly early when platform duration metadata is
// rounded or the last codec packet falls before the requested boundary. Accept
// at most half a second and 5% of the requested span, whichever is smaller.
// Keep the overshoot limit separate: stale source-duration metadata must never
// turn a short clip back into a full-length video.
fn duration_matches(expected: f64, actual: f64, source_known: bool, rounding: f64) -> bool {
    let shortfall = 0.5_f64.min(expected * 0.05);
    actual <= expected + rounding && (!source_known || actual >= expected - shortfall)
}

pub struct ClipJob {
    pub directory: PathBuf,
}

impl ClipJob {
    pub fn new(parent: &Path, id: u64) -> anyhow::Result<Self> {
        let directory = parent.join(format!(".omniget-clip-{id}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }
}

impl Drop for ClipJob {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct ReservedName(PathBuf);
impl ReservedName {
    fn commit(mut self) -> PathBuf {
        std::mem::take(&mut self.0)
    }
}
impl Drop for ReservedName {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}

fn safe_time_suffix(suffix: &str, restricted_storage: bool) -> String {
    if restricted_storage {
        suffix.replace(':', "-")
    } else {
        suffix.to_owned()
    }
}

fn reserve_name(
    parent: &Path,
    title: &str,
    range: &ClipRange,
    ext: &str,
) -> anyhow::Result<ReservedName> {
    // Keep the user's title, but never let title text become a path or a
    // yt-dlp template. Reserve room for range, duplicate counter and extension.
    let mut title = sanitize_filename::sanitize_with_options(
        title,
        sanitize_filename::Options {
            windows: cfg!(any(windows, target_os = "android")),
            ..Default::default()
        },
    )
    .trim_matches([' ', '.'])
    .to_string();
    if title.is_empty() {
        title = "video".into();
    }
    while title.len() > 120 {
        title.pop();
    }
    let range_suffix = range.suffix();
    let range_suffix = safe_time_suffix(&range_suffix, cfg!(any(windows, target_os = "android")));
    std::fs::create_dir_all(parent)?;
    for number in 0..100_000 {
        let duplicate = if number == 0 {
            String::new()
        } else {
            format!(" ({number})")
        };
        let path = parent.join(format!("{title}{range_suffix}{duplicate}.{ext}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(ReservedName(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Cannot reserve clip filename: {}", path.display()))
            }
        }
    }
    bail!("Too many clips with the same title and range")
}

/// Decode/re-encode the downloaded section rather than trusting inherited
/// container timestamps or duration tags. Video is H.264/AAC MP4 for sharing;
/// audio-only jobs retain their chosen audio format. No source file is replaced.
pub async fn finish(
    input: &Path,
    destination: &Path,
    title: &str,
    range: &ClipRange,
    source_duration: Option<f64>,
    token: &CancellationToken,
) -> anyhow::Result<(PathBuf, f64)> {
    if token.is_cancelled() {
        bail!("Download cancelled");
    }
    let expected = range.expected_duration(source_duration)?;
    let probe = super::ffmpeg::probe(input)
        .await
        .context("Cannot inspect downloaded clip")?;
    let audio_file = input
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "mp3" | "m4a" | "aac" | "opus" | "ogg" | "flac" | "wav"));
    let video = !audio_file
        && probe
            .streams
            .iter()
            .any(|s| s.codec_type == "video" && s.width.unwrap_or(0) > 0);
    let audio = probe.streams.iter().any(|s| s.codec_type == "audio");
    if !video && !audio {
        bail!("The downloaded clip has no playable audio or video");
    }
    let input_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("m4a")
        .to_ascii_lowercase();
    let ext = if video { "mp4" } else { input_ext.as_str() };
    let output = input
        .parent()
        .ok_or_else(|| anyhow!("Clip folder missing"))?
        .join(format!("normalized.{ext}"));
    let mut args: Vec<String> = ["-nostdin", "-y", "-fflags", "+genpts", "-i"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    args.push(input.to_string_lossy().into_owned());
    args.extend(
        ["-map_metadata", "-1", "-map_chapters", "-1"]
            .into_iter()
            .map(str::to_owned),
    );
    if video {
        args.extend(
            [
                "-map",
                "0:v:0",
                "-vf",
                "setpts=PTS-STARTPTS,pad=ceil(iw/2)*2:ceil(ih/2)*2",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "18",
                "-pix_fmt",
                "yuv420p",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
    if audio {
        let codec = if video {
            "aac"
        } else {
            match ext {
                "mp3" => "libmp3lame",
                "flac" => "flac",
                "wav" => "pcm_s16le",
                "ogg" => "libvorbis",
                "opus" => "libopus",
                _ => "aac",
            }
        };
        args.extend(
            [
                "-map",
                "0:a:0",
                "-af",
                "asetpts=PTS-STARTPTS,aresample=async=1:first_pts=0",
                "-c:a",
                codec,
            ]
            .into_iter()
            .map(str::to_owned),
        );
        if !matches!(codec, "flac" | "pcm_s16le") {
            args.extend(["-b:a".into(), "192k".into()]);
        }
    }
    if let Some(duration) = expected {
        args.extend(["-t".into(), format!("{duration:.3}")]);
    }
    if matches!(ext, "mp4" | "m4a" | "mov") {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
    args.push(output.to_string_lossy().into_owned());
    let mut command = super::process::command("ffmpeg");
    command.args(&args).kill_on_drop(true);
    let encoded = tokio::select! {
        result = command.output() => result.context("Could not normalize clip")?,
        _ = token.cancelled() => bail!("Download cancelled"),
    };
    if !encoded.status.success() {
        bail!(
            "Clip normalization failed: {}",
            String::from_utf8_lossy(&encoded.stderr)
                .chars()
                .rev()
                .take(1400)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    }
    let checked = super::ffmpeg::probe(&output)
        .await
        .context("Cannot validate normalized clip")?;
    let duration = checked.duration_seconds;
    if !duration.is_finite() || duration <= 0.0 {
        bail!("Normalized clip has no valid duration");
    }
    // A clip cannot claim the original video's duration or stop well before the
    // requested end. Keep codec/frame agreement and overshoot checks strict;
    // bounded section shortfalls are handled separately below.
    let tolerance = 0.3_f64.max(
        checked
            .streams
            .iter()
            .filter_map(|s| s.fps)
            .filter(|v| *v > 0.0)
            .map(|v| 2.0 / v)
            .fold(0.0, f64::max),
    );
    if let Some(expected) = expected {
        if !duration_matches(expected, duration, source_duration.is_some(), tolerance) {
            bail!("Clip duration mismatch: expected {expected:.3}s, got {duration:.3}s");
        }
    }
    for stream in &checked.streams {
        if let Some(d) = stream.duration_seconds {
            if (d - duration).abs() > tolerance {
                bail!("Clip stream and container durations disagree");
            }
        }
    }
    if token.is_cancelled() {
        bail!("Download cancelled");
    }
    let mut named_range = range.clone();
    if let Some(end) = range.clamped_end(source_duration) {
        named_range.end = Some(end);
    }
    let reserved = reserve_name(destination, title, &named_range, ext)?;
    // The placeholder belongs to this job alone. Copy before advertising the
    // final path; no other job can reserve or overwrite it while we finish.
    // Rename on the same filesystem publishes a complete file atomically.
    std::fs::rename(&output, &reserved.0).with_context(|| {
        format!(
            "Cannot publish normalized clip from {} to {}",
            output.display(),
            reserved.0.display()
        )
    })?;
    let path = reserved.commit();
    Ok((path, duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_are_validated_and_clamped_to_real_end() {
        let range = ClipRange::parse("3:00-6:00").unwrap();
        assert_eq!(range.expected_duration(Some(300.0)).unwrap(), Some(120.0));
        assert!(range.expected_duration(Some(100.0)).is_err());
        let past_end = ClipRange::parse("3-17").unwrap();
        assert_eq!(past_end.expected_duration(Some(15.0)).unwrap(), Some(12.0));
        assert_eq!(past_end.clamped_end(Some(15.0)), Some(15.0));
        assert_eq!(past_end.clamped_end(None), None);
        assert_eq!(past_end.clamped_end(Some(f64::NAN)), None);
        assert_eq!(past_end.clamped_end(Some(20.0)), None);
        assert_eq!(
            ClipRange::parse("30-inf").unwrap().suffix(),
            " (from 0:30 to end)"
        );
        for bad in [
            "5-4",
            "1:99-4:00",
            "a-b",
            "1..2-3",
            "-3",
            "1:2:3:4-500",
            "NaN-inf",
        ] {
            assert!(ClipRange::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn section_duration_allows_small_shortfalls_without_hiding_broken_clips() {
        // Device report: YouTube AV1, requested 12s, decoded media 11.652s.
        assert!(duration_matches(12.0, 11.652, true, 0.3));
        assert!(duration_matches(12.0, 12.0, true, 0.3));
        assert!(!duration_matches(12.0, 11.0, true, 0.3));
        assert!(!duration_matches(12.0, 15.0, true, 0.3));
        assert!(!duration_matches(12.0, 1800.0, true, 0.3));
        // Half a second is not acceptable for a one-second clip.
        assert!(!duration_matches(1.0, 0.652, true, 0.3));
        assert!(duration_matches(1.0, 0.975, true, 0.3));
        // Unknown source duration retains EOF support, not overshoot permission.
        assert!(duration_matches(12.0, 8.0, false, 0.3));
        assert!(!duration_matches(12.0, 1800.0, false, 0.3));
    }
    #[test]
    fn android_shared_storage_rejects_no_colons_in_clip_suffix() {
        assert_eq!(
            safe_time_suffix(" (from 0:30 to 1:45)", true),
            " (from 0-30 to 1-45)"
        );
        assert_eq!(
            safe_time_suffix(" (from 0:30 to 1:45)", false),
            " (from 0:30 to 1:45)"
        );
    }
    #[test]
    fn numbering_is_atomic_and_never_replaces_an_existing_clip() {
        let root = std::env::temp_dir().join(format!("clip-test-{}", uuid::Uuid::new_v4()));
        let range = ClipRange::parse("0:30-1:45").unwrap();
        let first = reserve_name(&root, "title", &range, "mp4").unwrap();
        let second = reserve_name(&root, "title", &range, "mp4").unwrap();
        let third = reserve_name(&root, "title", &range, "mp4").unwrap();
        let colon = if cfg!(any(windows, target_os = "android")) {
            "-"
        } else {
            ":"
        };
        assert_eq!(
            first.0.file_name().unwrap().to_str().unwrap(),
            format!("title (from 0{colon}30 to 1{colon}45).mp4")
        );
        assert!(second.0.to_string_lossy().ends_with(" (1).mp4"));
        assert!(third.0.to_string_lossy().ends_with(" (2).mp4"));
        drop((first, second, third));
        std::fs::remove_dir(root).unwrap();
    }
}
