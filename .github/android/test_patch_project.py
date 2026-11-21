#!/usr/bin/env python3
"""Offline regressions for the generated Android runtime integration."""
import io
import pathlib
import runpy
import tempfile
import unittest
import unittest.mock
import zipfile

PATCHER = pathlib.Path(__file__).with_name("patch_project.py")


class RuntimeBundleTests(unittest.TestCase):
    def run_patcher(self, archive, expected_hash):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            app = root / "src-tauri/gen/android/app"
            kotlin = app / "src/main/java/com/openselena/libreget/MainActivity.kt"
            kotlin.parent.mkdir(parents=True)
            kotlin.write_text(
                "package com.openselena.libreget\n"
                "import android.os.Bundle\n"
                "class MainActivity : TauriActivity() {\n"
                "  override fun onCreate(savedInstanceState: Bundle?) {\n"
                "    super.onCreate(savedInstanceState)\n"
                "  }\n"
                "}\n"
            )
            (app / "build.gradle.kts").write_text("android { defaultConfig {} }\ndependencies {}\n")
            (app / "src/main/AndroidManifest.xml").write_text(
                '<manifest><application>'
                '<activity android:launchMode="singleTask" android:name=".MainActivity" android:exported="true">'
                '<intent-filter><action android:name="android.intent.action.MAIN" />'
                '<category android:name="android.intent.category.LAUNCHER" /></intent-filter>'
                "</activity></application></manifest>"
            )
            (root / "src-tauri/libreget-core/src/core").mkdir(parents=True)
            (root / "src-tauri/icons").mkdir(parents=True)
            (root / "src-tauri/icons/icon.png").write_bytes(b"icon fixture")
            original = pathlib.Path.cwd()
            try:
                import os
                os.chdir(root)
                with unittest.mock.patch("subprocess.run"), \
                     unittest.mock.patch("urllib.request.urlopen", return_value=io.BytesIO(archive)), \
                     unittest.mock.patch("hashlib.sha256") as digest:
                    digest.return_value.hexdigest.return_value = expected_hash
                    runpy.run_path(str(PATCHER))
                manifest = (app / "src/main/AndroidManifest.xml").read_text()
                installer = (app / "src/main/java/com/openselena/libreget/ApkInstaller.kt").read_text()
                return kotlin.read_text(), (app / "src/main/res/raw/omniget_ytdlp").read_bytes(), manifest, installer
            finally:
                os.chdir(original)

    def archive(self, include_solver=True):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as archive:
            archive.writestr("yt_dlp/version.py", "__version__ = '2026.10.26'")
            if include_solver:
                for name in ("core", "lib"):
                    archive.writestr(f"yt_dlp_ejs/yt/solver/{name}.min.js", "// fixture")
        return data.getvalue()

    def test_verified_bundle_replaces_old_script_and_clears_cache_before_ready(self):
        expected = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3"
        archive = self.archive()
        kotlin, packaged, manifest, installer = self.run_patcher(archive, expected)
        self.assertEqual(packaged, archive)
        self.assertIn("android.intent.action.SEND", manifest)
        self.assertIn('android:mimeType="text/plain"', manifest)
        self.assertIn("REQUEST_INSTALL_PACKAGES", manifest)
        self.assertIn("androidx.core.content.FileProvider", manifest)
        self.assertIn("FILE_PROVIDER_PATHS", manifest)
        self.assertIn("@xml/file_paths", manifest)
        self.assertIn("package com.openselena.libreget", installer)
        self.assertIn("object ApkInstaller", installer)
        self.assertIn("getUriForFile", installer)
        self.assertIn("nativeOnSharedText", kotlin)
        self.assertIn("handleShareIntent(intent)", kotlin)
        self.assertIn("onNewIntent", kotlin)
        self.assertIn(expected, kotlin)
        self.assertIn("noBackupFilesDir", kotlin)
        self.assertLess(kotlin.index("deleteRecursively()"), kotlin.index("pending.renameTo(script)"))
        self.assertLess(kotlin.index("pending.renameTo(script)"), kotlin.index('".bundle-sha256"'))
        self.assertLess(kotlin.index('".bundle-sha256"'), kotlin.index('"runtime-ready"'))
        self.assertIn("check(digest(pending) == expectedHash)", kotlin)

    def test_bad_checksum_fails_closed(self):
        with self.assertRaisesRegex(SystemExit, "checksum mismatch"):
            self.run_patcher(self.archive(), "bad-hash")

    def test_missing_solver_fails_closed(self):
        expected = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3"
        with self.assertRaisesRegex(SystemExit, "missing required asset"):
            self.run_patcher(self.archive(False), expected)


if __name__ == "__main__":
    unittest.main()
