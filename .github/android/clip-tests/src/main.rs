mod process {
 pub fn command(p: &str) -> tokio::process::Command { tokio::process::Command::new(p) }
}
mod ffmpeg {
 pub struct Stream { pub codec_type:String,pub width:Option<u32>,pub fps:Option<f64>,pub duration_seconds:Option<f64> }
 pub struct Probe { pub streams:Vec<Stream>, pub duration_seconds:f64 }
 pub async fn probe(p: &std::path::Path)->anyhow::Result<Probe> {
  let x=tokio::process::Command::new("ffprobe").args(["-v","error","-show_format","-show_streams","-of","json"]).arg(p).output().await?;
  anyhow::ensure!(x.status.success(),"probe failed"); let j:serde_json::Value=serde_json::from_slice(&x.stdout)?;
  Ok(Probe {duration_seconds:j["format"]["duration"].as_str().unwrap_or("0").parse()?,streams:j["streams"].as_array().unwrap().iter().map(|s|Stream{codec_type:s["codec_type"].as_str().unwrap_or("").into(),width:s["width"].as_u64().map(|v|v as u32),fps:None,duration_seconds:s["duration"].as_str().and_then(|v|v.parse().ok())}).collect()})
 }
}
#[path="../../../../src-tauri/src/core/clip.rs"] mod clip;
#[tokio::main]
async fn main()->anyhow::Result<()> {
 let a:Vec<String>=std::env::args().collect(); let range=clip::ClipRange::parse(&a[3]).map_err(anyhow::Error::msg)?;
 let (path,duration)=clip::finish(std::path::Path::new(&a[1]),std::path::Path::new(&a[2]),"Test title",&range,Some(a[4].parse()?),&tokio_util::sync::CancellationToken::new()).await?;
 println!("{}\n{}",path.display(),duration); Ok(())
}
