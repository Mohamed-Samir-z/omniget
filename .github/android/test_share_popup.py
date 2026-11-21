#!/usr/bin/env python3
"""Native share host and JNI contract checks. UI behavior has Robolectric tests."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[2]
class NativeShareTests(unittest.TestCase):
 def test_cold_and_warm_use_one_runtime(self):
  p=(ROOT/'.github/android/patch_project.py').read_text()
  self.assertIn('handleShareIntent(intent)',p)
  self.assertIn('override fun onNewIntent',p)
  self.assertIn('override fun onWebViewCreate',p)
  self.assertIn('if (shareContentHidden) setShareContentVisible(false)',p)
  self.assertIn('sharePopup?.dispose()',p)
  self.assertNotIn('SYSTEM_ALERT_WINDOW',p)
  self.assertIn('android:windowIsTranslucent',p)
  self.assertIn('intent?.action != android.content.Intent.ACTION_SEND && !prefs.getBoolean',p)
 def test_only_explicit_full_app_action_prefills(self):
  rust=(ROOT/'src-tauri/src/android_share.rs').read_text()
  self.assertIn('action == "prefill"',rust)
  self.assertIn('prefill_shared_url(&app, url)',rust)
  self.assertIn('commands::downloads::download_from_url',rust)
  self.assertNotIn('queue_url_with_defaults',rust)
  external=(ROOT/'src-tauri/src/external_url.rs').read_text()
  part=external[external.index('pub async fn prefill_shared_url'):external.index('pub async fn register_frontend')]
  self.assertIn('action: "prefill"',part)
  self.assertNotIn('handle_external_url',part)
 def test_popup_waits_and_ignores_stale_callbacks(self):
  popup=(ROOT/'.github/android/SharePopup.kt').read_text()
  self.assertIn('nativeShareReady()',popup)
  self.assertIn('token != generation',popup)
  self.assertIn('Thread {',popup)
  self.assertIn('Handler(Looper.getMainLooper()).post',popup)
  self.assertIn('activity.moveTaskToBack(true)',popup)
  self.assertIn('runAction("prefill")',popup)
  self.assertIn('mode = if (mp3.isChecked) "mp3"',popup)
if __name__=='__main__':unittest.main()
