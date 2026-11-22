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

fn workspace_lock_result(result: Result<(), fs4::TryLockError>) -> anyhow::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(fs4::TryLockError::WouldBlock) => bail!("This clip is already running"),
        Err(fs4::TryLockError::Error(error)) => {
            bail!("Cannot lock clip workspace safely: {error}. Saved source was not deleted")
        }
    }
}

#[cfg(any(target_os = "android", test))]
fn unsupported_workspace_lock(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::Unsupported || matches!(error.raw_os_error(), Some(38 | 95))
}

/// OS lock whose ownership ends with the job, not inherited handles.
struct ClipLock(std::fs::File);

impl Drop for ClipLock {
    fn drop(&mut self) {
        // Closing our handle alone does not release an advisory lock while a
        // concurrent fork still has the same open-file description. End the
        // job's ownership explicitly, including early-return error paths.
        let _ = fs4::FileExt::unlock(&self.0);
    }
}

/// Durable job workspace. A completed source survives interruption; only a
/// verified final publish or an explicit discard may remove it.
pub struct ClipJob {
    pub directory: PathBuf,
    _locks: Vec<ClipLock>,
}
impl ClipJob {
    pub fn open(parent: &Path, key: &str) -> anyhow::Result<Self> {
        #[cfg(target_os = "android")]
        {
            let data = crate::core::paths::app_data_dir()
                .context("Cannot find private clip lock directory; saved source was not deleted")?;
            return Self::open_with_private_lock(parent, key, &data.join("clip-locks"));
        }
        #[cfg(not(target_os = "android"))]
        Self::open_in_workspace(parent, key)
    }
    #[cfg(not(target_os = "android"))]
    fn open_in_workspace(parent: &Path, key: &str) -> anyhow::Result<Self> {
        use fs4::FileExt;
        let key = uuid::Uuid::parse_str(key).context("Invalid clip recovery identity")?;
        let directory = parent.join(format!(".omniget-clip-{key}"));
        std::fs::create_dir_all(&directory)?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("workspace.lock"))?;
        workspace_lock_result(FileExt::try_lock(&lock))?;
        Ok(Self {
            directory,
            _locks: vec![ClipLock(lock)],
        })
    }
    /// Private sidecar files stay in place after completion. Unlinking a lock
    /// inode would let a new worker bypass another process already holding it.
    #[cfg(any(target_os = "android", test))]
    fn open_with_private_lock(parent: &Path, key: &str, lock_root: &Path) -> anyhow::Result<Self> {
        Self::open_with_private_lock_using(parent, key, lock_root, fs4::FileExt::try_lock)
    }
    #[cfg(any(target_os = "android", test))]
    fn open_with_private_lock_using(
        parent: &Path,
        key: &str,
        lock_root: &Path,
        legacy_try_lock: impl FnOnce(&std::fs::File) -> Result<(), fs4::TryLockError>,
    ) -> anyhow::Result<Self> {
        use fs4::FileExt;
        let key = uuid::Uuid::parse_str(key).context("Invalid clip recovery identity")?;
        std::fs::create_dir_all(lock_root)?;
        let private_lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_root.join(format!("{key}.lock")))?;
        workspace_lock_result(FileExt::try_lock(&private_lock))?;
        let private_lock = ClipLock(private_lock);
        let directory = parent.join(format!(".omniget-clip-{key}"));
        std::fs::create_dir_all(&directory)?;
        // Retain compatibility with a live older worker on filesystems where
        // the old workspace lock actually works. Only unsupported flock may
        // use the private lock alone, never contention or permission errors.
        let legacy = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("workspace.lock"))?;
        let mut locks = vec![private_lock];
        match legacy_try_lock(&legacy) {
            Ok(()) => locks.push(ClipLock(legacy)),
            Err(fs4::TryLockError::Error(error)) if unsupported_workspace_lock(&error) => {}
            Err(error) => workspace_lock_result(Err(error))?,
        }
        Ok(Self {
            directory,
            _locks: locks,
        })
    }
    /// A complete source also retains its original media description so a
    /// cold restart need not contact the platform before final processing.
    pub fn save_metadata(&self, value: &serde_json::Value) -> anyhow::Result<()> {
        let pending = self.directory.join("media-info.pending");
        use std::io::Write;
        let mut file = std::fs::File::create(&pending)?;
        file.write_all(&serde_json::to_vec(value)?)?;
        file.sync_all()?;
        std::fs::rename(pending, self.directory.join("media-info.json"))?;
        Ok(())
    }
    pub fn saved_metadata(parent: &Path, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let key = uuid::Uuid::parse_str(key)?;
        let directory = parent.join(format!(".omniget-clip-{key}"));
        if !directory.join("source-checkpoint.json").is_file() {
            return Ok(None);
        }
        let path = directory.join("media-info.json");
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }
    pub fn discard_if_idle(parent: &Path, key: &str) -> anyhow::Result<()> {
        let key = uuid::Uuid::parse_str(key)?;
        if !parent.join(format!(".omniget-clip-{key}")).exists() {
            return Ok(());
        }
        Self::open(parent, &key.to_string())?.complete()
    }
    /// The marker is written only after the downloader and all metadata steps
    /// return successfully, never inferred from a .part filename or size.
    pub fn save_source(&self, source: &Path, identity: &str, duration: f64) -> anyhow::Result<()> {
        let name = source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid clip source name"))?;
        anyhow::ensure!(
            source.parent() == Some(self.directory.as_path()),
            "Clip source is outside its workspace"
        );
        let hash = source_hash(source)?;
        let value = serde_json::json!({"version":1,"identity":identity,"name":name,"sha256":hash,"duration":duration});
        let pending = self.directory.join("source-checkpoint.pending");
        let mut file = std::fs::File::create(&pending)?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec(&value)?)?;
        file.sync_all()?;
        std::fs::rename(pending, self.directory.join("source-checkpoint.json"))?;
        Ok(())
    }
    pub fn source(&self, identity: &str) -> anyhow::Result<Option<(PathBuf, f64)>> {
        let marker = self.directory.join("source-checkpoint.json");
        if !marker.exists() {
            return Ok(None);
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(marker)?)?;
        anyhow::ensure!(
            value["version"] == 1 && value["identity"].as_str() == Some(identity),
            "Clip recovery choices do not match this workspace"
        );
        let name = value["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid clip checkpoint"))?;
        anyhow::ensure!(
            Path::new(name).components().count() == 1
                && !name.contains(['/', '\\'])
                && name != "."
                && name != "..",
            "Invalid clip checkpoint filename"
        );
        let path = self.directory.join(name);
        anyhow::ensure!(
            value["sha256"].as_str() == Some(source_hash(&path)?.as_str()),
            "Saved clip source changed; it was not reused"
        );
        Ok(Some((path, value["duration"].as_f64().unwrap_or(0.0))))
    }
    pub fn complete(self) -> anyhow::Result<()> {
        std::fs::remove_dir_all(&self.directory).context("Cannot clean finished clip workspace")
    }
}
fn source_hash(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
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
fn subtitle_time(value: f64) -> String {
    let ms = (value.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

fn trim_srt(text: &str, start: f64, duration: f64) -> anyhow::Result<String> {
    let normalized = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut output = String::new();
    let mut number = 0;
    for block in normalized.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let mut lines = block.lines();
        let first = lines.next().unwrap_or_default();
        let timing = if first.contains("-->") {
            first
        } else {
            lines
                .next()
                .ok_or_else(|| anyhow!("Invalid source subtitle timing"))?
        };
        let (a, b) = timing
            .split_once("-->")
            .ok_or_else(|| anyhow!("Invalid source subtitle timing"))?;
        let a = seconds(&a.trim().replace(',', "."))
            .ok_or_else(|| anyhow!("Invalid subtitle start"))?;
        let b = seconds(
            &b.split_whitespace()
                .next()
                .unwrap_or_default()
                .replace(',', "."),
        )
        .ok_or_else(|| anyhow!("Invalid subtitle end"))?;
        if b <= start || a >= start + duration {
            continue;
        }
        number += 1;
        output.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            number,
            subtitle_time((a - start).max(0.0)),
            subtitle_time((b - start).min(duration)),
            lines.collect::<Vec<_>>().join("\n")
        ));
    }
    Ok(output)
}

/// The extractor writes source-timeline subtitles, even for download sections.
/// Keep selected sidecars with the final clip, trimmed and rebased to zero.
async fn finish_subtitles(
    input: &Path,
    final_path: &Path,
    start: f64,
    duration: f64,
    token: &CancellationToken,
) -> anyhow::Result<()> {
    let Some(parent) = input.parent() else {
        return Ok(());
    };
    let source_stem = input
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("source");
    let target_stem = final_path
        .file_stem()
        .and_then(|v| v.to_str())
        .ok_or_else(|| anyhow!("Missing clip name"))?;
    let mut published = Vec::new();
    let result = async {
        for entry in std::fs::read_dir(parent)? {
            let path = entry?.path();
            let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
                continue;
            };
            if !matches!(ext, "srt" | "vtt" | "ass" | "ssa") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|v| v.to_str()) else {
                continue;
            };
            let Some(suffix) = stem.strip_prefix(source_stem) else {
                continue;
            };
            if !suffix.is_empty() && !suffix.starts_with('.') {
                continue;
            }
            // This destination is unique to the already-reserved clip name.
            let target = final_path.with_file_name(format!("{target_stem}{suffix}.{ext}"));
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .context("A subtitle file already uses this clip name")?;
            published.push(target.clone());
            if ext == "srt" {
                if token.is_cancelled() {
                    bail!("Download cancelled");
                }
                let text =
                    std::fs::read_to_string(&path).context("Cannot read source subtitles")?;
                std::fs::write(&target, trim_srt(&text, start, duration)?)?;
                continue;
            }
            let mut command = super::process::command("ffmpeg");
            command
                .args(["-nostdin", "-y", "-i"])
                .arg(&path)
                .args([
                    "-ss",
                    &format!("{start:.3}"),
                    "-t",
                    &format!("{duration:.3}"),
                    "-map",
                    "0:s:0",
                ])
                .arg(&target)
                .kill_on_drop(true);
            let output = tokio::select! {
                result = command.output() => result?,
                _ = token.cancelled() => bail!("Download cancelled"),
            };
            if !output.status.success() {
                bail!("Could not preserve the selected clip subtitles");
            }
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        for path in published {
            let _ = std::fs::remove_file(path);
        }
    }
    result
}

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
    let ext = if video {
        if input_ext == "mkv"
            && probe
                .streams
                .iter()
                .filter(|s| s.codec_type == "audio")
                .count()
                > 1
        {
            "mkv"
        } else {
            "mp4"
        }
    } else {
        input_ext.as_str()
    };
    let output = input
        .parent()
        .ok_or_else(|| anyhow!("Clip folder missing"))?
        .join(format!("normalized.{ext}"));
    let mut args: Vec<String> = ["-nostdin", "-y", "-fflags", "+genpts"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    // A phone must not allocate decoder/encoder workers for every CPU while
    // another link is being shared or downloaded. Quality/resolution stay put.
    #[cfg(target_os = "android")]
    args.extend([
        "-threads".into(),
        "2".into(),
        "-filter_threads".into(),
        "1".into(),
    ]);
    args.extend(["-i".into(), input.to_string_lossy().into_owned()]);
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
    #[cfg(target_os = "android")]
    if video {
        args.extend(["-threads:v".into(), "2".into()]);
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
                "0:a",
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
    if video && audio {
        for (index, stream) in probe
            .streams
            .iter()
            .filter(|s| s.codec_type == "audio")
            .enumerate()
        {
            args.extend([
                format!("-map_metadata:s:a:{index}"),
                format!("0:s:{}", stream.index),
            ]);
            args.extend([
                format!("-disposition:a:{index}"),
                if index == 0 { "default" } else { "0" }.into(),
            ]);
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
    if checked
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
        .count()
        != probe
            .streams
            .iter()
            .filter(|s| s.codec_type == "audio")
            .count()
    {
        bail!("Clip audio track count changed during normalization");
    }
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
    finish_subtitles(input, &reserved.0, range.start, duration, token).await?;
    let path = reserved.commit();
    Ok((path, duration))
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!("clip-checkpoint-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn successful_source_survives_drop_and_reopens_without_download() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        {
            let job = ClipJob::open(&root, &key).unwrap();
            let input = job.directory.join("source.mkv");
            std::fs::write(&input, b"downloaded-source").unwrap();
            job.save_source(&input, "exact-choices", 363.0).unwrap();
        }
        let job = ClipJob::open(&root, &key).unwrap();
        let (input, duration) = job.source("exact-choices").unwrap().unwrap();
        assert_eq!(std::fs::read(input).unwrap(), b"downloaded-source");
        assert_eq!(duration, 363.0);
        job.complete().unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn metadata_is_recoverable_only_after_a_completed_source() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        let metadata = serde_json::json!({"title":"Saved title", "duration_seconds":363.0});
        job.save_metadata(&metadata).unwrap();
        assert!(ClipJob::saved_metadata(&root, &key).unwrap().is_none());
        let input = job.directory.join("source.mkv");
        std::fs::write(&input, b"complete").unwrap();
        job.save_source(&input, "choices", 363.0).unwrap();
        drop(job);
        assert_eq!(
            ClipJob::saved_metadata(&root, &key).unwrap(),
            Some(metadata)
        );
        ClipJob::discard_if_idle(&root, &key).unwrap();
        assert!(ClipJob::saved_metadata(&root, &key).unwrap().is_none());
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn discard_never_deletes_a_running_workspace() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        assert!(ClipJob::discard_if_idle(&root, &key).is_err());
        assert!(job.directory.is_dir());
        drop(job);
        ClipJob::discard_if_idle(&root, &key).unwrap();
        assert!(!root.join(format!(".omniget-clip-{key}")).exists());
        assert!(ClipJob::discard_if_idle(&root, "../outside").is_err());
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn partial_transfer_is_not_falsely_reused() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        std::fs::write(job.directory.join("source.mkv.part"), b"partial").unwrap();
        assert!(job.source("choices").unwrap().is_none());
        assert!(job.directory.join("source.mkv.part").exists());
        job.complete().unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn changed_source_and_choices_fail_closed() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        let input = job.directory.join("source.mkv");
        std::fs::write(&input, b"good").unwrap();
        job.save_source(&input, "choices", 10.0).unwrap();
        assert!(job.source("other-choices").is_err());
        std::fs::write(&input, b"evil").unwrap();
        assert!(job.source("choices").is_err());
        job.complete().unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn duplicate_workers_do_not_share_mutable_source() {
        let root = root();
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        assert!(ClipJob::open(&root, &key).is_err());
        drop(job);
        let job = ClipJob::open(&root, &key).unwrap();
        job.complete().unwrap();
        std::fs::remove_dir(root).unwrap();
    }
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

#[cfg(test)]
mod subtitle_tests {
    use super::*;
    #[test]
    fn srt_is_trimmed_rebased_and_clamped_without_ffmpeg_version_dependence() {
        let input="1\r\n00:00:00,000 --> 00:00:01,500\r\nFirst\r\n\r\n2\r\n00:00:02,000 --> 00:00:04,000\r\nSecond\r\n";
        let output = trim_srt(input, 1.0, 2.0).unwrap();
        assert!(output.contains("00:00:00,000 --> 00:00:00,500\nFirst"));
        assert!(output.contains("00:00:01,000 --> 00:00:02,000\nSecond"));
        assert!(trim_srt("bad subtitle", 1.0, 2.0).is_err());
    }
}

#[cfg(test)]
mod lock_error_tests {
    use super::*;
    #[test]
    fn only_contention_claims_already_running() {
        assert!(workspace_lock_result(Ok(())).is_ok());
        assert_eq!(
            workspace_lock_result(Err(fs4::TryLockError::WouldBlock))
                .unwrap_err()
                .to_string(),
            "This clip is already running"
        );
        for kind in [
            std::io::ErrorKind::Unsupported,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::Other,
        ] {
            let message =
                workspace_lock_result(Err(fs4::TryLockError::Error(std::io::Error::from(kind))))
                    .unwrap_err()
                    .to_string();
            assert!(!message.contains("already running"));
            assert!(message.contains("Saved source was not deleted"));
        }
        let message = workspace_lock_result(Err(fs4::TryLockError::Error(
            std::io::Error::from_raw_os_error(38),
        )))
        .unwrap_err()
        .to_string();
        assert!(message.contains("os error 38"));
    }
}

#[cfg(test)]
mod private_lock_tests {
    use super::*;
    #[test]
    fn private_lock_retains_completed_source_and_sidecar() {
        let root = std::env::temp_dir().join(format!("private-clip-{}", uuid::Uuid::new_v4()));
        let output = root.join("output");
        let locks = root.join("private");
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open_with_private_lock(&output, &key, &locks).unwrap();
        let source = job.directory.join("source.mkv");
        std::fs::write(&source, b"saved-source").unwrap();
        job.save_source(&source, "same", 5.0).unwrap();
        assert!(ClipJob::open_with_private_lock(&output, &key, &locks).is_err());
        assert!(ClipJob::open(&output, &key).is_err());
        drop(job);
        let job = ClipJob::open_with_private_lock(&output, &key, &locks).unwrap();
        assert!(job.source("same").unwrap().is_some());
        job.complete().unwrap();
        assert!(locks.join(format!("{key}.lock")).exists());
        let job = ClipJob::open_with_private_lock(&output, &key, &locks).unwrap();
        job.complete().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn old_worker_contention_never_bypassed_by_private_lock() {
        let root = std::env::temp_dir().join(format!("legacy-clip-{}", uuid::Uuid::new_v4()));
        let key = uuid::Uuid::new_v4().to_string();
        let job = ClipJob::open(&root, &key).unwrap();
        assert!(ClipJob::open_with_private_lock(&root, &key, &root.join("private")).is_err());
        drop(job);
        let job = ClipJob::open_with_private_lock(&root, &key, &root.join("private")).unwrap();
        job.complete().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn fallback_is_only_unsupported_not_contention_or_permissions() {
        assert!(unsupported_workspace_lock(
            &std::io::Error::from_raw_os_error(38)
        ));
        assert!(unsupported_workspace_lock(
            &std::io::Error::from_raw_os_error(95)
        ));
        assert!(!unsupported_workspace_lock(
            &std::io::Error::from_raw_os_error(11)
        ));
        assert!(!unsupported_workspace_lock(
            &std::io::Error::from_raw_os_error(13)
        ));
    }
}

#[cfg(test)]
mod unsupported_fs_integration {
    use super::*;
    #[test]
    fn unsupported_shared_lock_keeps_real_private_exclusion_and_checkpoint() {
        let root = std::env::temp_dir().join(format!("enosys-clip-{}", uuid::Uuid::new_v4()));
        let output = root.join("shared");
        let private = root.join("private");
        let key = uuid::Uuid::new_v4().to_string();
        let enosys = |_: &std::fs::File| {
            Err(fs4::TryLockError::Error(std::io::Error::from_raw_os_error(
                38,
            )))
        };
        let job = ClipJob::open_with_private_lock_using(&output, &key, &private, enosys).unwrap();
        let source = job.directory.join("source.mkv");
        std::fs::write(&source, b"retained").unwrap();
        job.save_source(&source, "same", 4.0).unwrap();
        assert!(ClipJob::open_with_private_lock_using(&output, &key, &private, enosys).is_err());
        drop(job);
        let denied = |_: &std::fs::File| {
            Err(fs4::TryLockError::Error(std::io::Error::from_raw_os_error(
                13,
            )))
        };
        assert!(ClipJob::open_with_private_lock_using(&output, &key, &private, denied).is_err());
        assert!(source.is_file());
        let job = ClipJob::open_with_private_lock_using(&output, &key, &private, enosys).unwrap();
        assert_eq!(
            std::fs::read(job.source("same").unwrap().unwrap().0).unwrap(),
            b"retained"
        );
        job.complete().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod inherited_lock_regression {
    use super::*;

    #[test]
    fn inherited_descriptors_do_not_keep_a_finished_job_locked() {
        let root = std::env::temp_dir().join(format!("inherited-clip-{}", uuid::Uuid::new_v4()));
        let key = uuid::Uuid::new_v4().to_string();
        let output = root.join("output");
        let private = root.join("private");
        let job = ClipJob::open_with_private_lock(&output, &key, &private).unwrap();
        // dup and fork retain the same open-file description. A concurrent
        // subprocess may briefly inherit it before its close-on-exec runs.
        let inherited_private = job._locks[0].0.try_clone().unwrap();
        let inherited_legacy = job._locks[1].0.try_clone().unwrap();
        drop(job);
        let reopened = ClipJob::open_with_private_lock(&output, &key, &private);
        assert!(
            reopened.is_ok(),
            "Finished job remained locked: {:?}",
            reopened.err()
        );
        drop(inherited_private);
        drop(inherited_legacy);
        std::fs::remove_dir_all(root).unwrap();
    }
        }
