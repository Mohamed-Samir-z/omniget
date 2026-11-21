import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
const root = path.resolve(__dirname, "../../..");
const read = (p: string) => fs.readFileSync(path.join(root, p), "utf8");
describe("multiple source audio tracks", () => {
 it("uses separate audio-only source tracks and binds the full language array", () => {
  const selector = read("src/components/librebox/FormatSelector.svelte");
  expect(selector).toContain("f.has_audio && !f.has_video && f.audio_language");
  expect(selector).toContain("selectedAudioLanguages.includes(language)");
  expect(selector).toContain("[...selectedAudioLanguages, language]");
  const home=read("src/routes/+page.svelte");
  expect(home).toContain("bind:multipleAudio");
  expect(home).toContain("multipleAudio && selectedAudioLanguages.length < 2");
  expect(home).toContain("audioLanguages: multipleAudio ? selectedAudioLanguages : null");
  expect(home).toContain("selectedAudioLanguages = [];\n    multipleAudio = false;");
 });
 it("preserves choices in recovery and prevents lossy fallback", () => {
  expect(read("src-tauri/src/core/queue_wal.rs")).toContain("pub audio_languages: Option<Vec<String>>");
  const ytdlp=read("src-tauri/libreget-core/src/core/ytdlp.rs");
  expect(ytdlp).toContain("--audio-multistreams");
  expect(ytdlp).toContain("Merger+ffmpeg_o:{labels}");
  expect(ytdlp).toContain("refusing a single-track fallback");
  expect(read("src-tauri/src/core/clip.rs")).toContain('"0:a"');
  expect(read("src-tauri/src/core/queue.rs")).toContain("ffmpeg::finalize_audio_tracks");
 });
});
