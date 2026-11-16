//! No-op stand-in for `hotkey.rs` on platforms without global shortcuts
//! (Android). Global hotkeys do not exist on a phone, so settings that would
//! re-register them simply have nothing to do.

pub fn reregister(_app: &tauri::AppHandle) {}

pub fn register_from_settings(_app: &tauri::AppHandle) {}
