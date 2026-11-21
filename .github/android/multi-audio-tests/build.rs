use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../")
        .canonicalize()
        .unwrap();
    let source = root.join("src-tauri/libreget-core/src/core/ffmpeg.rs");
    let s = fs::read_to_string(&source).unwrap();
    let structs = &s[s
        .find("#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct MediaProbeInfo")
        .unwrap()
        ..s.find("#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct ConversionResult")
            .unwrap()];
    let probe =
        &s[s.find("pub async fn probe(").unwrap()..s.find("pub async fn get_duration_us").unwrap()];
    let finalizer = &s[s.find("fn canonical_audio_language").unwrap()..];
    let finalizer = finalizer.replace(
        "include_str!(\"audio_language_codes.json\")",
        &format!(
            "include_str!({:?})",
            source.with_file_name("audio_language_codes.json")
        ),
    );
    let ytdlp = fs::read_to_string(root.join("src-tauri/libreget-core/src/core/ytdlp.rs")).unwrap();
    let selector = &ytdlp[ytdlp.find("pub fn is_valid_audio_language").unwrap()
        ..ytdlp
            .find("#[allow(clippy::too_many_arguments)]\npub async fn download_video(")
            .unwrap()];
    let tests = &ytdlp[ytdlp.find("#[cfg(test)]\nmod multi_audio_tests").unwrap()..];
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("production.rs");
    fs::write(out,format!("use std::path::Path; use anyhow::anyhow; use serde::{{Serialize, Deserialize}};\n{structs}\n{probe}\n{finalizer}\n{selector}\n{tests}")).unwrap();
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src-tauri/libreget-core/src/core/ytdlp.rs")
            .display()
    );
}
