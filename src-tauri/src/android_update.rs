//! Android in-app updates from GitHub Releases.
//!
//! Desktop uses tauri-plugin-updater, which is not linked into the Android
//! build. Android instead checks the `android-latest` prerelease on GitHub:
//! CI stamps every build with `LIBREGET_BUILD_TIME` (UTC ISO-8601), so the
//! asset time and installed source commit identify newer approved builds.
//! The APK downloads to the app cache (SHA-256 verification is mandatory) and the system package
//! installer opens it through the `ApkInstaller` Kotlin helper that
//! `.github/android/patch_project.py` injects into the generated project.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

static VERIFIED_APK: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);
static UPDATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const RELEASE_API: &str =
    "https://api.github.com/repos/Mohamed-Samir-z/omniget/releases/tags/android-latest";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AndroidUpdateInfo {
    pub available: bool,
    /// Why no update is offered: "dev-build" (no CI stamp) or "up-to-date".
    pub reason: Option<String>,
    pub title: Option<String>,
    pub published_at: Option<String>,
    pub download_url: Option<String>,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
    pub build_time: Option<String>,
}

/// First 64-char hex token in free-form text. Release notes carry the APK
/// checksum as `SHA-256: <hex>`; commit short-hashes are too short to match.
fn extract_sha256(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .find(|token| token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

fn valid_apk_url(url: &str) -> bool {
    url.starts_with("https://github.com/Mohamed-Samir-z/omniget/releases/download/")
        && url.ends_with("/libreget-android.apk")
        && !url.contains('?')
        && !url.contains('#')
}

/// UTC ISO-8601 stamp written into the build by CI (`LIBREGET_BUILD_TIME`).
/// Dev builds have no stamp and never report an update.
pub fn build_time() -> Option<String> {
    option_env!("LIBREGET_BUILD_TIME").map(str::to_string)
}

#[tauri::command]
pub async fn check_android_update() -> Result<AndroidUpdateInfo, String> {
    let build_time = build_time();
    let client = reqwest::Client::builder()
        .user_agent("omniget-android-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(RELEASE_API)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("release check failed with HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let published_at = body
        .get("published_at")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let title = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let notes = body.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let mut sha256 = extract_sha256(notes);
    let mut asset_time = None;
    let mut download_url = None;
    let mut size_bytes = None;
    if let Some(assets) = body.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name == "libreget-android.apk" {
                asset_time = asset.get("created_at").and_then(|v| v.as_str());
                if let Some(digest) = asset
                    .get("digest")
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.strip_prefix("sha256:"))
                {
                    sha256 = extract_sha256(digest);
                }
                download_url = asset
                    .get("browser_download_url")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                size_bytes = asset.get("size").and_then(|v| v.as_u64());
                break;
            }
        }
    }
    if sha256.is_none()
        || download_url
            .as_deref()
            .is_none_or(|url| !valid_apk_url(url))
        || size_bytes.is_none_or(|size| size == 0)
    {
        return Err("The approved release is missing a valid APK or checksum. Retry later".into());
    }
    // Publication can happen after the installed build was compiled. Never
    // offer the same commit again when an exact tested artifact is promoted.
    let installed_sha = option_env!("LIBREGET_BUILD_SHA").unwrap_or("");
    let same_build = !installed_sha.is_empty()
        && (notes.contains(installed_sha)
            || body.get("target_commitish").and_then(|v| v.as_str()) == Some(installed_sha));
    let available = !same_build
        && sha256.is_some()
        && match (
            &build_time,
            asset_time.or(published_at.as_deref()),
            &download_url,
        ) {
            (Some(stamp), Some(created), Some(url)) => {
                created > stamp.as_str() && valid_apk_url(url)
            }
            _ => false,
        };
    let reason = if build_time.is_none() {
        Some("dev-build".to_string())
    } else if !available {
        Some("up-to-date".to_string())
    } else {
        None
    };
    Ok(AndroidUpdateInfo {
        available,
        reason,
        title,
        published_at,
        download_url,
        size_bytes,
        sha256,
        build_time,
    })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: u64,
}

/// Downloads the APK to the app cache directory, reporting
/// `android-update-progress` events, and requires a release SHA-256 match. Returns the local path on success; partial
/// or corrupted downloads are deleted and reported as errors.
#[tauri::command]
pub async fn download_android_update(
    app: AppHandle,
    url: String,
    sha256: Option<String>,
) -> Result<String, String> {
    use futures::StreamExt;
    use sha2::Digest;
    use tokio::io::AsyncWriteExt;

    let _guard = UPDATE_LOCK.lock().await;
    if !crate::network_policy::downloads_allowed(&app) {
        return Err("Waiting for Wi-Fi before downloading the update".to_string());
    }
    *VERIFIED_APK.lock().map_err(|e| e.to_string())? = None;
    let release = check_android_update().await?;
    let expected = release
        .sha256
        .ok_or("The approved release has no checksum")?;
    if !release.available
        || release.download_url.as_deref() != Some(url.as_str())
        || sha256.as_deref() != Some(expected.as_str())
        || !valid_apk_url(&url)
    {
        return Err("The approved update changed. Check for updates again".to_string());
    }
    let client = reqwest::Client::builder()
        .user_agent("omniget-android-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!(
            "update download failed with HTTP {}",
            resp.status()
        ));
    }
    let total = resp.content_length().unwrap_or(0);
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let path = dir.join("omniget-update.apk");
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        if downloaded - last_emit >= 512 * 1024 || downloaded == total {
            last_emit = downloaded;
            if !crate::network_policy::downloads_allowed(&app) {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err("Wi-Fi disconnected. Retry the update when Wi-Fi returns".to_string());
            }
            let _ = app.emit(
                "android-update-progress",
                UpdateProgress { downloaded, total },
            );
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    if total != 0 && downloaded != total {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(format!(
            "incomplete download: {downloaded} of {total} bytes"
        ));
    }
    {
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected.to_ascii_lowercase() {
            let _ = tokio::fs::remove_file(&path).await;
            return Err("checksum mismatch; the download was corrupted".to_string());
        }
    }
    let path = path.to_string_lossy().to_string();
    *VERIFIED_APK.lock().map_err(|e| e.to_string())? = Some((path.clone(), expected));
    Ok(path)
}

/// Opens the downloaded APK in the system package installer, which shows
/// the usual Android install confirmation over the existing app.
#[tauri::command]
pub async fn install_android_update(path: String) -> Result<(), String> {
    use sha2::Digest;
    let _guard = UPDATE_LOCK.lock().await;
    let (verified_path, expected) = VERIFIED_APK
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Download and verify the update first")?;
    if path != verified_path {
        return Err("Unverified APK path".to_string());
    }
    let verify_path = path.clone();
    let actual = tokio::task::spawn_blocking(move || -> Result<String, String> {
        use std::io::Read;
        let mut file = std::fs::File::open(verify_path).map_err(|e| e.to_string())?;
        let mut hash = sha2::Sha256::new();
        let mut buffer = [0_u8; 65536];
        loop {
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        Ok(format!("{:x}", hash.finalize()))
    })
    .await
    .map_err(|e| e.to_string())??;
    if actual != expected {
        return Err("The downloaded APK changed; download it again".to_string());
    }
    install_apk(path).await
}

#[cfg(target_os = "android")]
async fn install_apk(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
        let context = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
        // Threads attached from Rust see the boot classloader, which cannot
        // see app classes, so load ApkInstaller through the app's own
        // classloader (same approach as the download service bridge).
        let loader = env
            .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
            .map_err(|e| e.to_string())?
            .l()
            .map_err(|e| e.to_string())?;
        let name = env
            .new_string("com.openselena.libreget.ApkInstaller")
            .map_err(|e| e.to_string())?;
        let class = env
            .call_method(
                &loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[jni::objects::JValue::Object(&name)],
            )
            .map_err(|e| e.to_string())?
            .l()
            .map_err(|e| e.to_string())?;
        let class = jni::objects::JClass::from(class);
        let jpath = env.new_string(&path).map_err(|e| e.to_string())?;
        let result = env.call_static_method(
            class,
            "install",
            "(Landroid/content/Context;Ljava/lang/String;)V",
            &[
                jni::objects::JValue::Object(&context),
                jni::objects::JValue::Object(&jpath),
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let _ = env.exception_clear();
                Err(e.to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(not(target_os = "android"))]
async fn install_apk(_path: String) -> Result<(), String> {
    Err("APK install is only available on Android".to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_sha256;

    #[test]
    fn finds_checksum_in_release_notes() {
        let notes = "Debug-quality build of commit a64c145.\n\
                     SHA-256: `48ca1b18e5a5d50293efc907b21479f2ac76e21db2b2b8392d02c59d79976cd0`\n\
                     Install over the existing app.";
        assert_eq!(
            extract_sha256(notes),
            Some("48ca1b18e5a5d50293efc907b21479f2ac76e21db2b2b8392d02c59d79976cd0".to_string())
        );
    }

    #[test]
    fn ignores_short_hex_like_commit_hashes() {
        assert_eq!(extract_sha256("build of commit a64c145"), None);
    }
}
