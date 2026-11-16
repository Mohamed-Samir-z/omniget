//! Android process bootstrap.
//!
//! An Android app process has no `HOME`, no `XDG_*` variables, a read-only
//! working directory and a `temp_dir()` it cannot write to. The rest of the
//! code base resolves every location through the `dirs` crate (which on
//! Android falls back to its XDG implementation) and `std::env::temp_dir`,
//! so instead of touching hundreds of call sites this module points those
//! variables at the app's own sandbox before anything else runs.
//!
//! Layout, with `<files>` = `/data/data/<package>/files`:
//!
//! - `HOME`            -> `<files>`
//! - `XDG_DATA_HOME`   -> `<files>/data`   (settings, databases, plugins)
//! - `XDG_CONFIG_HOME` -> `<files>/config` (holds `user-dirs.dirs`)
//! - `XDG_CACHE_HOME`  -> `<package>/cache`
//! - `TMPDIR`          -> `<package>/cache`
//! - download dir      -> see [`pick_downloads_dir`]

use std::path::{Path, PathBuf};

/// The package name is the process name, which Android exposes as the first
/// NUL-terminated entry of `/proc/self/cmdline` (secondary processes append
/// `:suffix`).
fn package_name() -> Option<String> {
    let raw = std::fs::read("/proc/self/cmdline").ok()?;
    let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
    let name = String::from_utf8_lossy(&raw[..end]).to_string();
    let name = name.split(':').next()?.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    match std::fs::write(&probe, b"x") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Where finished downloads go by default, best location first:
///
/// 1. `/storage/emulated/0/Download/OmniGet` - visible in any file manager and
///    the gallery, but only writable once the user has granted "All files
///    access" (or on older Android versions).
/// 2. `/storage/emulated/0/Android/data/<package>/files/Download` - always
///    writable without permissions, but hidden by the Files app on Android 11+.
/// 3. `<files>/Download` inside the private sandbox - the last resort.
fn pick_downloads_dir(package: &str, files: &Path) -> PathBuf {
    let candidates = [
        PathBuf::from("/storage/emulated/0/Download/OmniGet"),
        PathBuf::from(format!(
            "/storage/emulated/0/Android/data/{package}/files/Download"
        )),
    ];
    for candidate in candidates {
        if is_writable(&candidate) {
            return candidate;
        }
    }
    let fallback = files.join("Download");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

pub fn init() {
    let Some(package) = package_name() else {
        return;
    };
    let internal = [
        format!("/data/user/0/{package}"),
        format!("/data/data/{package}"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.is_dir());
    let Some(internal) = internal else {
        return;
    };

    let files = internal.join("files");
    let cache = internal.join("cache");
    let data = files.join("data");
    let config = files.join("config");
    for dir in [&files, &cache, &data, &config] {
        let _ = std::fs::create_dir_all(dir);
    }

    let downloads = pick_downloads_dir(&package, &files);

    // `dirs::download_dir()` and friends read these from `user-dirs.dirs`
    // under `$XDG_CONFIG_HOME`.
    let user_dirs = format!(
        "XDG_DOWNLOAD_DIR=\"{d}\"\nXDG_VIDEOS_DIR=\"{d}\"\nXDG_MUSIC_DIR=\"{d}\"\nXDG_DOCUMENTS_DIR=\"{d}\"\n",
        d = downloads.display()
    );
    let _ = std::fs::write(config.join("user-dirs.dirs"), user_dirs);

    std::env::set_var("HOME", &files);
    std::env::set_var("XDG_DATA_HOME", &data);
    std::env::set_var("XDG_CONFIG_HOME", &config);
    std::env::set_var("XDG_CACHE_HOME", &cache);
    std::env::set_var("TMPDIR", &cache);
}
