#!/usr/bin/env python3
"""Regressions for first-byte progress on Android's urllib-only runtime.

Pass --yt-dlp PATH to also exercise the pinned zipapp against a slow local
HTTP server. No YouTube credentials or external network access are needed.
"""
import http.server
import pathlib
import subprocess
import sys
import tempfile
import threading
import time
import unittest

YTDLP = None
if "--yt-dlp" in sys.argv:
    index = sys.argv.index("--yt-dlp")
    YTDLP = str(pathlib.Path(sys.argv[index + 1]).resolve())
    del sys.argv[index:index + 2]


class SlowMedia(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "video/mp4")
        self.send_header("Content-Length", str(16 * 4096))
        self.end_headers()
        try:
            for _ in range(16):
                self.wfile.write(b"x" * 4096)
                self.wfile.flush()
                time.sleep(0.1)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, *_):
        pass


class StartupTests(unittest.TestCase):
    def test_android_uses_small_adaptive_initial_buffer(self):
        # The buffer change is Android-only and applied during CI patching.
        source = pathlib.Path(__file__).with_name("startup.patch").read_text()
        self.assertIn('if cfg!(target_os = "android") { "1K" } else { "16M" }', source)
        self.assertNotIn('"--no-resize-buffer".to_string()', source)

    def test_watchdog_starts_at_media_not_extraction(self):
        patch = pathlib.Path(__file__).with_name("startup.patch").read_text()
        self.assertIn('line.starts_with("[debug] Invoking ")', patch)
        self.assertIn('line.contains(" downloader on ")', patch)
        self.assertIn('from_secs(60)', patch)
        self.assertIn('child.kill().await', patch)
        self.assertIn('media startup timed out', patch)
        self.assertIn('*pending.lock().unwrap() = None', patch)
        self.assertIn('parse_progress_line(&line).is_some()', patch)

    @unittest.skipUnless(YTDLP, "pass --yt-dlp PATH for local HTTP integration test")
    def test_small_buffer_reports_before_slow_response_finishes(self):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), SlowMedia)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            durations = {}
            for buffer in ("16M", "1K"):
                with tempfile.TemporaryDirectory() as directory:
                    started = time.monotonic()
                    process = subprocess.Popen([
                        sys.executable, YTDLP, "--ignore-config", "--proxy", "",
                        "--no-warnings", "--no-quiet", "--progress", "--newline",
                        "--progress-template", "download:bytes:%(progress.downloaded_bytes)s",
                        "--socket-timeout", "0.5", "--buffer-size", buffer,
                        "-o", str(pathlib.Path(directory) / "video.mp4"),
                        f"http://127.0.0.1:{server.server_port}/video.mp4",
                    ], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                    first = None
                    output = []
                    try:
                        for line in process.stdout:
                            output.append(line)
                            if line.startswith("bytes:") and first is None:
                                first = time.monotonic() - started
                        self.assertEqual(process.wait(timeout=10), 0, "".join(output))
                    finally:
                        if process.poll() is None:
                            process.kill()
                            process.wait()
                        process.stdout.close()
                    self.assertIsNotNone(first, "".join(output))
                    durations[buffer] = first
            print("First progress on trickling HTTP response:", durations)
            # A huge read waits for the entire response, despite a 0.5s socket
            # timeout, because each individual receive gets bytes in time.
            self.assertGreater(durations["16M"], 1.4)
            self.assertLess(durations["1K"], durations["16M"] - 0.8)
        finally:
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    unittest.main()
