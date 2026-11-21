mod core {
    pub mod process {
        pub fn command(p: &str) -> tokio::process::Command {
            tokio::process::Command::new(p)
        }
    }
}
include!(concat!(env!("OUT_DIR"), "/production.rs"));
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    finalize_audio_tracks(std::path::Path::new(&a[1]), &a[2..]).await
}
