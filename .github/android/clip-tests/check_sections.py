from pathlib import Path
import subprocess,threading,http.server,functools,json,tempfile
with tempfile.TemporaryDirectory() as d:
 p=Path(d);out=p/'out';out.mkdir()
 subprocess.run(['ffmpeg','-v','error','-f','lavfi','-i','testsrc2=size=160x90:rate=12','-f','lavfi','-i','sine=frequency=440:sample_rate=44100','-t','45','-c:v','libx264','-g','120','-keyint_min','120','-sc_threshold','0','-c:a','aac','-shortest',str(p/'sample.mp4')],check=True)
 class Handler(http.server.BaseHTTPRequestHandler):
  def log_message(self,*args): pass
  def do_HEAD(self): self.respond(False)
  def do_GET(self): self.respond(True)
  def respond(self,body):
   data=(p/'sample.mp4').read_bytes();start=0;end=len(data)-1
   range=self.headers.get('Range')
   if range:
    lo,hi=range.removeprefix('bytes=').split('-');start=int(lo or 0);end=int(hi) if hi else end
   self.send_response(206 if range else 200);self.send_header('Content-Type','video/mp4');self.send_header('Accept-Ranges','bytes');self.send_header('Content-Length',str(end-start+1))
   if range:self.send_header('Content-Range',f'bytes {start}-{end}/{len(data)}')
   self.end_headers()
   if body:
    try:self.wfile.write(data[start:end+1])
    except (BrokenPipeError,ConnectionResetError):pass
 server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
 try:
  subprocess.run(['yt-dlp','--no-playlist','--download-sections','*0:13-0:23','-o',str(out/'%(title)s.%(ext)s'),f'http://127.0.0.1:{server.server_address[1]}/sample.mp4'],check=True,capture_output=True,text=True)
  files=list(out.glob('*'));assert len(files)==1
  x=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_format','-of','json',str(files[0])]))
  print('Downloaded section duration:',x['format']['duration'])
  subprocess.run([str(Path(__file__).parent/'target/debug/clip-harness'),str(files[0]),str(p/'final'),'0:13-0:23','45'],check=True)
  clip=list((p/'final').glob('*'))[0]
  import array
  def frame(path,at):
   return list(subprocess.check_output(['ffmpeg','-v','error','-ss',str(at),'-i',str(path),'-frames:v','1','-f','rawvideo','-pix_fmt','rgb24','-']))
  actual=frame(clip,0)
  scores={t:round(sum((a-b)**2 for a,b in zip(actual,frame(p/'sample.mp4',t)))/len(actual),2) for t in [10,12,13,14]}
  print('First-frame MSE by source timestamp:',scores)
  assert min(scores,key=scores.get)==13,scores
  print(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration,start_time:stream=duration,start_time','-of','json',str(clip)],text=True))
 finally:server.shutdown();thread.join()
