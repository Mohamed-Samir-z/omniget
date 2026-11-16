//! `trash::delete` with a fallback for platforms that have no system trash.
//!
//! Android has no Recycle Bin / Trash directory that an app can move files
//! into, and the `trash` crate does not build for it. There the file is
//! removed permanently, which is the only honest behaviour available.

use std::path::Path;

#[cfg(not(target_os = "android"))]
pub fn delete<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    trash::delete(path.as_ref()).map_err(|e| std::io::Error::other(e.to_string()))
}

#[cfg(target_os = "android")]
pub fn delete<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let path = path.as_ref();
    if path.is_dir() && !path.is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}
