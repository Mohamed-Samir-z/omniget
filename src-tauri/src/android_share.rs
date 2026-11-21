//! Android share-target bridge.
//!
//! `MainActivity` receives `ACTION_SEND` intents (the user picked omniget in
//! the Android share sheet) and forwards the shared text here through JNI.
//! The shared text usually mixes the URL with commentary, so we pull out the
//! first http(s) link and hand it to the external-URL pipeline, which queues
//! it until the frontend is ready and then opens the compact share sheet.

#![cfg(target_os = "android")]

use std::sync::OnceLock;

use jni::objects::{JObject, JString};
use jni::JNIEnv;
use tauri::AppHandle;

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

/// JNI entry point called by `MainActivity.onCreate` / `onNewIntent`.
///
/// The symbol name encodes the application id, so `.github/android/
/// patch_project.py` asserts at CI time that the generated Kotlin package
/// matches `com.openselena.libreget`; a mismatch fails the build instead of
/// silently breaking the share target.
#[no_mangle]
pub extern "C" fn Java_com_openselena_libreget_MainActivity_nativeOnSharedText(
    mut env: JNIEnv,
    _activity: JObject,
    text: JString,
) {
    let shared: String = match env.get_string(&text) {
        Ok(s) => s.into(),
        Err(e) => {
            tracing::warn!("[android] share intent: unreadable text: {}", e);
            return;
        }
    };
    let Some(url) = extract_shared_url(&shared) else {
        tracing::warn!("[android] share intent had no http(s) URL");
        return;
    };
    let Some(app) = APP_HANDLE.get().cloned() else {
        tracing::warn!("[android] share intent before app handle was set");
        return;
    };
    tauri::async_runtime::spawn(async move {
        crate::external_url::share_received(&app, url).await;
    });
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
