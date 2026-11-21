"""Exercise strict source selection, mux, clip, labels, and failure paths offline."""
from pathlib import Path
import json, subprocess, tempfile, http.server, functools, threading
ROOT=Path(__file__).resolve().parents[3]
FINAL=Path(__file__).parent/'target/debug/multi-audio-harness'
CLIP=ROOT/'.github/android/clip-tests/target/debug/clip-harness'
def run(*args):
 try: return subprocess.check_output(list(map(str,args)),text=True,stderr=subprocess.PIPE)
 except subprocess.CalledProcessError as error:
  print(error.stderr)
  raise
def probe(path):
 return json.loads(run('ffprobe','-v','error','-show_streams','-show_format','-of','json',path))
def audios(path): return [s for s in probe(path)['streams'] if s['codec_type']=='audio']
with tempfile.TemporaryDirectory() as directory:
 root=Path(directory)
 class QuietHandler(http.server.SimpleHTTPRequestHandler):
  def log_message(self, *_): pass
 server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(QuietHandler,directory=directory))
 threading.Thread(target=server.serve_forever,daemon=True).start()
 base=f'http://127.0.0.1:{server.server_port}/' 
 video=root/'video.mp4'
 run('ffmpeg','-v','error','-f','lavfi','-i','color=size=160x90:rate=12','-t','3','-an','-c:v','libx264',video)
 formats=[dict(format_id='v',url=base+video.name,ext='mp4',vcodec='h264',acodec='none',width=160,height=90)]
 for i,(lang,freq) in enumerate([('ar',440),('en',660),('fr',880)]):
  path=root/f'{lang}.m4a'
  run('ffmpeg','-v','error','-f','lavfi','-i',f'sine=frequency={freq}','-t','3','-c:a','aac','-metadata:s:a:0',f'language={lang}',path)
  formats.append(dict(format_id=lang,url=base+path.name,ext='m4a',vcodec='none',acodec='aac',language=lang,abr=128))
 info=root/'info.json';info.write_text(json.dumps(dict(id='fixture',title='Three source languages',extractor='generic',webpage_url='https://example.invalid/fixture',duration=3,formats=formats)))
 choices=json.loads(run(FINAL,'--choices',info,'ar','en','de'))
 assert choices['available']==['ar','en'] and choices['unavailable']==['de'] and not choices['unknown']
 unknown=json.loads(run(FINAL,'--choices',root/'nonexistent.json','ar'))
 assert unknown['unknown']==['ar'] and not unknown['unavailable']
 target=root/'full.mkv'
 selector='bv[height<=720]+ba[language=ar]+ba[language=en]+ba[language=fr]'
 run('yt-dlp','--enable-file-urls','--load-info-json',info,'--no-simulate','--no-playlist','--audio-multistreams','--postprocessor-args','Merger+ffmpeg_o:-metadata:s:a:0 language=ar -metadata:s:a:1 language=en -metadata:s:a:2 language=fr','--merge-output-format','mkv','--remux-video','mkv','-f',selector,'-o',target)
 assert len(audios(target))==3

 run(FINAL,target,'ar','en','fr')
 tracks=audios(target)
 assert [a['tags']['language'] for a in tracks]==['ar','en','fr']
 assert [a['disposition']['default'] for a in tracks]==[1,0,0]
 # The metadata rewrite must map every stream before the final check.
 metadata=root/'metadata.mkv';run('ffmpeg','-v','error','-i',target,'-map','0','-c','copy','-metadata','title=Metadata regression',metadata)
 run(FINAL,metadata,'ar','en','fr')
 # Section downloads use FFmpegDownloader, not MergerPP. Verify that path too.
 section=root/'section.mkv'
 run('yt-dlp','--load-info-json',info,'--no-simulate','--no-playlist','--download-sections','*1-3','--audio-multistreams','--downloader-args','ffmpeg_o:-metadata:s:a:0 language=ar -metadata:s:a:1 language=en -metadata:s:a:2 language=fr','--merge-output-format','mkv','--remux-video','mkv','-f',selector,'-o',section)
 run(FINAL,section,'ar','en','fr')
 metadata=section
 # Source subtitles use original timestamps and must survive temporary cleanup.
 subtitle=metadata.with_suffix('.ar.srt')
 subtitle.write_text('1\n00:00:00,000 --> 00:00:00,500\nBefore\n\n2\n00:00:01,200 --> 00:00:02,000\nSelected Arabic source caption\n\n3\n00:00:02,800 --> 00:00:03,000\nAfter\n')
 clip=Path(run(CLIP,metadata,root/'clips','0:01-0:03','3').splitlines()[0]);assert clip.suffix=='.mkv'
 clipped_subtitle=clip.with_suffix('.ar.srt')
 assert clipped_subtitle.exists()
 text=clipped_subtitle.read_text()
 assert 'Before' not in text and '00:00:00,200 --> 00:00:01,000' in text and 'Selected Arabic source caption' in text
 subtitle.unlink()
 run(FINAL,clip,'ar','en','fr');tracks=audios(clip)
 assert len(tracks)==3 and [a['tags']['language'] for a in tracks]==['ar','en','fr']
 assert [a['disposition']['default'] for a in tracks]==[1,0,0]
 assert abs(float(probe(clip)['format']['duration'])-2)<0.3
 before=clip.read_bytes()
 for languages in [('ar','en'),('en','ar','fr')]:
  rejected=subprocess.run(list(map(str,[FINAL,clip,*languages])),capture_output=True,text=True)
  assert rejected.returncode!=0
  assert clip.read_bytes()==before
 assert not list(clip.parent.glob('.omniget-audio-*'))
 missing=subprocess.run(list(map(str,['yt-dlp','--enable-file-urls','--load-info-json',info,'--simulate','--audio-multistreams','-f','bv+ba[language=ar]+ba[language=de]'])),capture_output=True,text=True)
 assert missing.returncode!=0 and 'Requested format is not available' in missing.stderr
 print('PASS: real yt-dlp 3-source mux; full and clipped MKV; metadata; ordered languages/default; missing/reordered track rejection; no silent fallback')
