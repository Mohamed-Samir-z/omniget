//! Android share-target bridge.
//!
//! The native Android popup uses the same initialized queue as the full app.
//! It never forwards a quick download through clipboard auto-download settings.
//! Only the explicit More options action emits a prefill-only frontend event.

#![cfg(target_os = "android")]

use std::sync::OnceLock;

use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use tauri::{AppHandle, Manager};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

/// First http(s) token in free-form shared text, with surrounding punctuation
/// (parentheses, quotes) stripped.
fn extract_shared_url(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let trimmed = token
            .trim_matches(|c: char| matches!(c, '(' | ')' | '<' | '>' | '"' | '\'' | ',' | ';'));
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Readiness probe used only while the native popup is waiting on cold start.
#[no_mangle]
pub extern "C" fn Java_com_openselena_libreget_MainActivity_nativeShareReady(
    _env: JNIEnv,
    _activity: JObject,
) -> jboolean {
    if APP_HANDLE.get().is_some() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Called from a Kotlin worker, never the Android UI thread. An empty string
/// means the action finished; other strings are safe user-facing errors.
#[no_mangle]
pub extern "C" fn Java_com_openselena_libreget_MainActivity_nativeShareAction(
    mut env: JNIEnv,
    _activity: JObject,
    text: JString,
    mode: JString,
    quality: JString,
    action: JString,
) -> jstring {
    let result = (|| -> Result<(), String> {
        let shared: String = env
            .get_string(&text)
            .map_err(|_| "Cannot read shared text")?
            .into();
        let mode: String = env
            .get_string(&mode)
            .map_err(|_| "Cannot read download type")?
            .into();
        let quality: String = env
            .get_string(&quality)
            .map_err(|_| "Cannot read quality")?
            .into();
        let action: String = env
            .get_string(&action)
            .map_err(|_| "Cannot read share action")?
            .into();
        let url = extract_shared_url(&shared).ok_or("No HTTP or HTTPS video link was shared")?;
        let app = APP_HANDLE
            .get()
            .cloned()
            .ok_or("The app is still starting")?;
        if action == "prefill" {
            tauri::async_runtime::block_on(crate::external_url::prefill_shared_url(&app, url));
            return Ok(());
        }
        if action != "download" || !matches!(mode.as_str(), "video" | "mp3") {
            return Err("Unsupported share action".to_string());
        }
        if !matches!(
            quality.as_str(),
            "best" | "1080p" | "720p" | "480p" | "360p"
        ) {
            return Err("Unsupported video quality".to_string());
        }
        let settings = crate::storage::config::load_settings(&app);
        let output = settings
            .download
            .default_output_dir
            .to_string_lossy()
            .into_owned();
        if output.trim().is_empty() {
            return Err("Open omniget and choose a download folder first".to_string());
        }
        let state = app.state::<crate::AppState>();
        tauri::async_runtime::block_on(crate::commands::downloads::download_from_url(
            app.clone(),
            state,
            url,
            output,
            if mode == "mp3" { Some(mode) } else { None },
            Some(quality),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(String::new()),
        ))
        .map(|_| ())
    })();
    let message = result.err().unwrap_or_default();
    env.new_string(message)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::extract_shared_url;

    #[test]
    fn extracts_url_from_commentary() {
        assert_eq!(
            extract_shared_url("Check this out! https://www.youtube.com/watch?v=abc 🦆"),
            Some("https://www.youtube.com/watch?v=abc".to_string())
        );
    }

    #[test]
    fn strips_wrapping_punctuation() {
        assert_eq!(
            extract_shared_url("video: (https://tiktok.com/@a/video/1),"),
            Some("https://tiktok.com/@a/video/1".to_string())
        );
    }

    #[test]
    fn ignores_text_without_links() {
        assert_eq!(extract_shared_url("no links here"), None);
    }
}
