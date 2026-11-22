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

#[cfg(all(test, target_os = "linux"))]
mod lock_diagnostics {
 use super::*;
 use anyhow::Context;
 #[test]
 fn non_contention_error_is_currently_misreported_as_running() {
  let failure: Result<(),fs4::TryLockError> = Err(std::io::Error::from_raw_os_error(38).into());
  let error=failure.context("This clip is already running").unwrap_err();
  assert_eq!(error.to_string(),"This clip is already running");
  assert!(format!("{error:#}").contains("not implemented"));
 }
 #[test]
 fn killed_owner_leaves_file_but_not_lock_or_source_loss() {
  use std::io::{BufRead,BufReader};
  let root=std::env::temp_dir().join(format!("clip-killed-owner-{}",uuid::Uuid::new_v4()));
  let key=uuid::Uuid::new_v4().to_string();
  let directory;
  {let job=clip::ClipJob::open(&root,&key).unwrap();directory=job.directory.clone();let source=directory.join("source.mkv");std::fs::write(&source,b"saved-source").unwrap();job.save_source(&source,"choices",12.0).unwrap();}
  let code="import fcntl,sys,time; f=open(sys.argv[1],'r+'); fcntl.flock(f,fcntl.LOCK_EX); print('ready',flush=True); time.sleep(60)";
  let child=std::process::Command::new("python3").args(["-c",code]).arg(directory.join("workspace.lock")).stdout(std::process::Stdio::piped()).spawn();
  let mut child=child.unwrap();let mut ready=String::new();BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready).unwrap();assert_eq!(ready.trim(),"ready");
  assert!(clip::ClipJob::open(&root,&key).is_err());
  child.kill().unwrap();child.wait().unwrap();
  assert!(directory.join("workspace.lock").exists());
  let job=clip::ClipJob::open(&root,&key).unwrap();let (source,_) = job.source("choices").unwrap().unwrap();assert_eq!(std::fs::read(source).unwrap(),b"saved-source");
  job.complete().unwrap();std::fs::remove_dir(root).unwrap();
 }
}
