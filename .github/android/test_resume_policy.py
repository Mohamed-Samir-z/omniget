#!/usr/bin/env python3
"""Offline safety checks for Android resume and network policy wiring."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]

class ResumePolicyTests(unittest.TestCase):
    def test_cancel_and_retry_never_delete_other_jobs_partial_files(self):
        source = (ROOT / 'src-tauri/libreget-core/src/core/ytdlp.rs').read_text()
        self.assertNotIn('cleanup_part_files(', source)
        self.assertIn('Keep .part/.ytdl files', source)
        self.assertIn('.kill_on_drop(true)', source)

    def test_wifi_is_transport_not_metering(self):
        source = (ROOT / 'src-tauri/src/network_policy.rs').read_text()
        self.assertIn('"hasTransport"', source)
        self.assertIn('JValue::Int(1)', source)
        self.assertIn('check().unwrap_or(false)', source)
        patcher = (ROOT / '.github/android/patch_project.py').read_text()
        self.assertIn('ACCESS_NETWORK_STATE', patcher)

    def test_start_and_running_jobs_both_have_gate(self):
        source = (ROOT / 'src-tauri/src/core/queue.rs').read_text()
        self.assertIn('wait_for_network_loss(&app, &queue, item_id)', source)
        self.assertGreaterEqual(source.count('downloads_allowed(&app)'), 2)
        self.assertIn('let network_lost = tokio::select!', source)
        self.assertIn('if network_lost {', source)
        self.assertIn('let wifi_waiting = cfg!(target_os = "android");', source)

    def test_updater_requires_checksum_and_installed_identity(self):
        source = (ROOT / 'src-tauri/src/android_update.rs').read_text()
        self.assertIn('!same_build && sha256.is_some()', ' '.join(source.split()))
        self.assertIn('LIBREGET_BUILD_SHA', source)
        self.assertIn('VERIFIED_APK', source)
        self.assertIn('actual != expected', source)

if __name__ == '__main__':
    unittest.main()
