#[cfg(desktop)]
use tauri_plugin_autostart::ManagerExt;

#[cfg(desktop)]
pub fn apply_autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if std::env::var("LIBREGET_PORTABLE").is_ok() {
        return Ok(());
    }
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {e}"))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {e}"))?;
    }
    Ok(())
}

/// Phones have no "start with system" setting to apply.
#[cfg(not(desktop))]
pub fn apply_autostart(_app: &tauri::AppHandle, _enabled: bool) -> Result<(), String> {
    Ok(())
}
