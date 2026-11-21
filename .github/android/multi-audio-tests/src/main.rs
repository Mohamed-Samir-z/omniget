#[cfg(test)]
#[path = "../../../../src-tauri/src/core/multi_audio_policy.rs"]
mod multi_audio_policy;
mod core {
    pub mod process {
        pub fn command(p: &str) -> tokio::process::Command {
            tokio::process::Command::new(p)
        }
    }
}
include!(concat!(env!("OUT_DIR"), "/production.rs"));
// Run the copied production choice checker against local extractor fixtures.
async fn get_video_info(
    ytdlp: &std::path::Path,
    info: &str,
    flags: &[String],
) -> anyhow::Result<serde_json::Value> {
    let result = tokio::process::Command::new(ytdlp)
        .args([
            "--load-info-json",
            info,
            "--dump-single-json",
            "--skip-download",
            "--retries",
            "0",
        ])
        .args(flags)
        .output()
        .await?;
    if !result.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&result.stderr).to_string());
    }
    Ok(serde_json::from_slice(&result.stdout)?)
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    if a[1] == "--choices" {
        println!(
            "{}",
            audio_track_choices(std::path::Path::new("yt-dlp"), &a[2], &a[3..], None).await
        );
        return Ok(());
    }
    finalize_audio_tracks(std::path::Path::new(&a[1]), &a[2..]).await
}
