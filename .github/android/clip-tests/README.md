# Clip regression harness

This executable imports the application's actual `src-tauri/src/core/clip.rs`.
Only the process/probe adapters are replaced with local FFmpeg calls, so media
normalization and filename reservation run through production code.

Requirements: Rust, Python 3, FFmpeg/FFprobe and yt-dlp on PATH.

```sh
cargo test --manifest-path .github/android/clip-tests/Cargo.toml
cargo build --manifest-path .github/android/clip-tests/Cargo.toml
python3 .github/android/clip-tests/check_sections.py
python3 .github/android/clip-tests/check_normalization.py
python3 .github/android/clip-tests/check_recovery.py
```

The section test serves a controlled long-GOP source over HTTP and checks that
a non-keyframe 13s to 23s cut starts at the right source frame. The normalization
test checks a 60s clip carrying original 30-minute timestamps, parallel duplicate
names, MP3 audio-only output and an end past EOF. No network service is contacted
apart from the loopback fixture server.

The recovery test kills real eight-track FFmpeg normalization, removes the original
fixture input, then reuses the durable verified source checkpoint. It validates all
eight output tracks and duration. It does not emulate Android Activity lifecycle
or prove byte-range continuation during an unfinished network transfer.
