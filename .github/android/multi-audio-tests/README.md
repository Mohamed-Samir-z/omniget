# Multiple audio regression harness

The build script imports the exact production selector, probe and final audio
verifier. It avoids unrelated native desktop dependencies. The clip harness
imports the production normalizer as well.

Build both harnesses, then run `python3 check_multiaudio.py` from this directory.
FFmpeg, FFprobe and yt-dlp 2026.10.26 are required. All media is generated locally;
only file URLs are used. Tests cover three distinct source languages, strict
selection, a single MKV, metadata processing, clipping, ordered language labels,
first-audio default and fail-closed missing/reordered tracks.

`audio_language_codes.json` is an ISO 639-2 alias table generated from the
iso-codes distribution. It matches two-letter, terminology and bibliographic
language tags from media containers. Region subtags do not affect this identity
check; the requested full tag is preserved in the output.
