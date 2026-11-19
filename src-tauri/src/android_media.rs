//! Android-only helpers that talk to the Java side through JNI.
//!
//! Files the app writes straight into shared storage are invisible to the
//! gallery and to media players until the system media scanner indexes them,
//! so every finished download is handed to `MediaScannerConnection`. Opening a
//! folder uses the Files app's document URI so the "show in folder" button
//! works without a FileProvider.

use jni::errors::Result as JniResult;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;
use tauri::{AppHandle, Manager};

/// Runs `f` on the Android main activity with a JNI environment.
///
/// The closure is queued on the webview thread, so this returns as soon as the
/// work is scheduled; failures are logged rather than reported to the caller.
fn run_on_activity<F>(app: &AppHandle, what: &'static str, f: F) -> Result<(), String>
where
    F: FnOnce(&mut JNIEnv, &JObject) -> JniResult<()> + Send + 'static,
{
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not ready".to_string())?;
    window
        .with_webview(move |webview| {
            webview.jni_handle().exec(move |env, activity, _webview| {
                if let Err(e) = f(env, activity) {
                    tracing::warn!("[android] {} failed: {}", what, e);
                    let _ = env.exception_clear();
                }
            });
        })
        .map_err(|e| e.to_string())
}

/// Asks the system media scanner to index `path` (video, audio or image) so it
/// shows up in the gallery and in other players. Directories are ignored.
pub fn scan_file(app: &AppHandle, path: &std::path::Path) {
    if !path.is_file() {
        return;
    }
    let path = path.to_string_lossy().to_string();
    let _ = run_on_activity(app, "media scan", move |env, activity| {
        let jpath = env.new_string(&path)?;
        let string_class = env.find_class("java/lang/String")?;
        let paths = env.new_object_array(1, &string_class, &jpath)?;
        // Passing null for the MIME types lets the scanner infer them from
        // the file extension.
        let null = JObject::null();
        env.call_static_method(
            "android/media/MediaScannerConnection",
            "scanFile",
            "(Landroid/content/Context;[Ljava/lang/String;[Ljava/lang/String;Landroid/media/MediaScannerConnection$OnScanCompletedListener;)V",
            &[
                JValue::Object(activity),
                JValue::Object(&paths),
                JValue::Object(&null),
                JValue::Object(&null),
            ],
        )?;
        Ok(())
    });
}

/// Opens the folder that contains `path` in the system Files app.
pub fn open_folder(app: &AppHandle, path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let dir = if p.is_dir() {
        p
    } else {
        p.parent().unwrap_or(p)
    };
    let dir = dir.to_string_lossy();
    let relative = dir
        .strip_prefix("/storage/emulated/0/")
        .or_else(|| dir.strip_prefix("/sdcard/"))
        .ok_or_else(|| "this folder is in private app storage".to_string())?;
    let uri = format!(
        "content://com.android.externalstorage.documents/document/primary%3A{}",
        relative.replace('/', "%2F")
    );

    run_on_activity(app, "open folder", move |env, activity| {
        let juri = env.new_string(&uri)?;
        let parsed = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&juri)],
            )?
            .l()?;
        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let mime = env.new_string("vnd.android.document/directory")?;
        env.call_method(
            &intent,
            "setDataAndType",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&parsed), JValue::Object(&mime)],
        )?;
        // FLAG_ACTIVITY_NEW_TASK
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(0x1000_0000)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
}
