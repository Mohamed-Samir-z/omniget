#!/usr/bin/env python3
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[2]
class Build4Tests(unittest.TestCase):
 def test_mp3_is_per_job_and_converts_native_media(self):
  q=(ROOT/'src-tauri/src/core/queue.rs').read_text()
  self.assertIn('let force_mp3 = download_mode.as_deref() == Some("mp3")',q)
  self.assertIn('Some("mp3".to_string())',q)
  cmd=(ROOT/'src-tauri/src/commands/downloads.rs').read_text()
  self.assertIn('if force_mp3 {',cmd)
  self.assertIn('GenericYtdlpDownloader::new()',cmd)
  core=(ROOT/'src-tauri/libreget-core/src/platforms/generic_ytdlp.rs').read_text()
  self.assertIn('&& !converting_mp3',core)
 def test_notification_speed_bridge_agrees(self):
  patch=(ROOT/'.github/android/gallery.patch').read_text()
  self.assertIn('(Ljava/lang/String;IIJ)V',patch)
  self.assertIn('JValue::Long(speed.max(0.0) as i64)',patch)
  service=(ROOT/'.github/android/DownloadService.kt').read_text()
  self.assertIn('count: Int, speed: Long',service)
  self.assertIn('MB/s',service)
  self.assertIn('$detail',service)
 def test_share_schedule_has_validation_and_limit(self):
  share=(ROOT/'src/routes/share/+page.svelte').read_text()
  self.assertIn('type="datetime-local"',share)
  self.assertIn('schedule_app_required',share)
  self.assertIn('getTime() <= Date.now()',share)
  cmd=(ROOT/'src-tauri/src/commands/downloads.rs').read_text()
  self.assertIn('stop <= scheduled_at.unwrap_or_else(queue::now_ms)',cmd)
if __name__=='__main__':unittest.main()
