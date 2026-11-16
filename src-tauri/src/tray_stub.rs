//! No-op stand-in for `tray.rs` on platforms without a system tray (Android).
//!
//! Keeps the exact public surface of the desktop module so callers in the
//! queue, events and command layers compile unchanged. Nothing here touches
//! the UI: there is no tray menu, tooltip or taskbar badge on a phone.

use tauri::AppHandle;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    pub quit: String,
    pub channels: String,
    pub no_active_downloads: String,
    pub downloads_label: String,
    pub active_suffix: String,
    pub speed_unit_kb: String,
    pub speed_unit_mb: String,
    pub decimal_separator: String,
}

pub fn apply_labels(_app: &AppHandle, _new_labels: TrayLabels) {}

pub fn setup(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

pub fn rebuild_menu(
    _app: &AppHandle,
    _header: String,
    _channels: Vec<(String, String)>,
) -> tauri::Result<()> {
    Ok(())
}

pub fn update_active_count(_app: &AppHandle, _count: u32) {}

pub fn update_speed_tooltip(_app: &AppHandle, _count: u32, _total_speed_bps: f64) {}

pub fn compute_total_active(_app: &AppHandle) -> u32 {
    0
}

pub fn show_window(_app: &AppHandle) {}

pub fn request_quit(app: &AppHandle) {
    app.exit(0);
}

pub fn update_taskbar_badge(_app: &AppHandle, _active_count: u32, _avg_percent: f64) {}
