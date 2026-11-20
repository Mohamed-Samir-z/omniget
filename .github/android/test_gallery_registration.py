#!/usr/bin/env python3
"""Structural regressions for generated Android integration, after patching."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class GalleryIntegrationTests(unittest.TestCase):
    def test_final_media_is_registered_after_metadata_replacement(self):
        queue = (ROOT / 'src-tauri/src/core/queue.rs').read_text()
        self.assertEqual(queue.count('crate::android_media::scan_file('), 1)
        scan = queue.index('crate::android_media::scan_file(')
        self.assertLess(queue.index('ffmpeg::embed_metadata('), scan)
        self.assertLess(scan, queue.index('q.mark_complete(', scan))
        self.assertIn('spawn_blocking(move ||', queue)
        self.assertIn('MediaStore registered:', queue)
        self.assertIn('gallery registration failed:', queue)

    def test_scan_has_a_bounded_completion_check(self):
        helper = (Path(__file__).with_name('MediaLibrary.kt')).read_text()
        self.assertIn('OnScanCompletedListener', helper)
        self.assertIn('finished.await(12, TimeUnit.SECONDS)', helper)
        self.assertIn('repeat(2)', helper)
        self.assertLess(helper.index('result.get()?.let'), helper.index('return uri.toString()'))
        self.assertIn('notifyChange(uri, null)', helper)

    def test_every_direct_instagram_carousel_item_is_registered(self):
        queue = (ROOT / 'src-tauri/src/core/queue.rs').read_text()
        self.assertIn('scan_paths.push(path)', queue)
        self.assertIn('for scan_path in scan_paths', queue)

    def test_android_names_are_byte_bounded_and_unicode_safe(self):
        source = (ROOT / 'src-tauri/libreget-core/src/core/ytdlp.rs').read_text()
        self.assertIn('%(title,id|download).120B', source)
        self.assertIn('%(id).32B', source)
        self.assertNotIn('let max_name = 40_usize', source)


if __name__ == '__main__':
    unittest.main()
