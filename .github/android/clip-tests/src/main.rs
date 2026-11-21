#[cfg(test)]
#[path="../../../../src-tauri/src/core/queue_wal.rs"] pub mod queue_wal;
#[cfg(test)]
#[path="../../../../src-tauri/src/core/recovery.rs"] mod recovery;
#[cfg(test)]
mod core {
 pub use crate::queue_wal;
 pub mod paths { pub fn app_data_dir()->Option<std::path::PathBuf> { None } }
}
mod process {
 pub fn command(p: &str) -> tokio::process::Command { tokio::process::Command::new(p) }
}
mod ffmpeg {
 pub struct Stream { pub index:u32, pub codec_type:String,pub width:Option<u32>,pub fps:Option<f64>,pub duration_seconds:Option<f64> }
 pub struct Probe { pub streams:Vec<Stream>, pub duration_seconds:f64 }
 pub async fn probe(p: &std::path::Path)->anyhow::Result<Probe> {
  let x=tokio::process::Command::new("ffprobe").args(["-v","error","-show_format","-show_streams","-of","json"]).arg(p).output().await?;
  anyhow::ensure!(x.status.success(),"probe failed"); let j:serde_json::Value=serde_json::from_slice(&x.stdout)?;
  Ok(Probe {duration_seconds:j["format"]["duration"].as_str().unwrap_or("0").parse()?,streams:j["streams"].as_array().unwrap().iter().map(|s|Stream{index:s["index"].as_u64().unwrap_or(0) as u32,codec_type:s["codec_type"].as_str().unwrap_or("").into(),width:s["width"].as_u64().map(|v|v as u32),fps:None,duration_seconds:s["duration"].as_str().and_then(|v|v.parse().ok())}).collect()})
 }
}
#[path="../../../../src-tauri/src/core/clip.rs"] mod clip;
#[tokio::main]
async fn main()->anyhow::Result<()> {
 let a:Vec<String>=std::env::args().collect();
 if a.get(1).map(String::as_str)==Some("checkpoint") {
  let job=clip::ClipJob::open(std::path::Path::new(&a[2]),&a[3])?;
  let source=if let Some((path,_))=job.source("eight-track-fixture")? { eprintln!("REUSED_SOURCE"); path } else {
   let target=job.directory.join("source.mkv");std::fs::copy(&a[4],&target)?;job.save_metadata(&serde_json::json!({"title":"Eight-track fixture","duration":60}))?;job.save_source(&target,"eight-track-fixture",60.0)?;eprintln!("SAVED_SOURCE");target
  };
  let token=tokio_util::sync::CancellationToken::new();
  let (path,duration)=clip::finish(&source,std::path::Path::new(&a[5]),"Checkpoint fixture",&clip::ClipRange::parse("0-60").map_err(anyhow::Error::msg)?,Some(60.0),&token).await?;
  println!("{}\n{}",path.display(),duration);job.complete()?;return Ok(())
 }
 let range=clip::ClipRange::parse(&a[3]).map_err(anyhow::Error::msg)?;
 let (path,duration)=clip::finish(std::path::Path::new(&a[1]),std::path::Path::new(&a[2]),"Test title",&range,Some(a[4].parse()?),&tokio_util::sync::CancellationToken::new()).await?;
 println!("{}\n{}",path.display(),duration); Ok(())
}
