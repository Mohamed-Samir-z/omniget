"""Kill real normalization, reuse the verified eight-track source on restart.
No remote URLs, no phone required. Does not prove Activity/process continuity.
"""
import hashlib,json,os,signal,subprocess,tempfile,time,uuid
from pathlib import Path
BIN=Path(__file__).parent.resolve()/'target/debug/clip-harness'
with tempfile.TemporaryDirectory() as root:
 root=Path(root);source=root/'fixture.mkv';work=root/'work';final=root/'final';work.mkdir();final.mkdir()
 args=['ffmpeg','-v','error','-f','lavfi','-i','testsrc2=size=640x360:rate=24']
 for n in range(8):args+=['-f','lavfi','-i',f'sine=frequency={300+n*100}:sample_rate=48000']
 args+=['-t','60','-map','0:v']
 for n in range(8):args+=['-map',f'{n+1}:a']
 args+=['-c:v','libx264','-threads','2','-preset','ultrafast','-c:a','aac',str(source)]
 subprocess.run(args,check=True)
 key=str(uuid.uuid4());cmd=[str(BIN),'checkpoint',str(work),key,str(source),str(final)]
 with (root/'first.log').open('w') as log:
  job=subprocess.Popen(cmd,stdout=log,stderr=log,start_new_session=True)
  deadline=time.monotonic()+30
  while time.monotonic()<deadline:
   outputs=list(work.glob('*/normalized.*'))
   if outputs and outputs[0].stat().st_size>10000:break
   assert job.poll() is None,'fixture finished before interruption';time.sleep(.02)
  else:raise AssertionError('normalization did not begin')
  saved=next(work.glob('*/source.mkv'));before=hashlib.sha256(saved.read_bytes()).hexdigest()
  os.killpg(job.pid,signal.SIGKILL);job.wait()
 assert saved.exists(),'interruption lost downloaded source'
 source.unlink() # Any attempted new download/copy would now fail.
 result=subprocess.run(cmd,text=True,capture_output=True,check=True)
 assert 'REUSED_SOURCE' in result.stderr and 'SAVED_SOURCE' not in result.stderr,result.stderr
 output=Path(result.stdout.splitlines()[0]);assert output.exists()
 data=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_format','-show_streams','-of','json',str(output)],text=True))
 audios=[s for s in data['streams'] if s['codec_type']=='audio'];assert len(audios)==8
 assert abs(float(data['format']['duration'])-60)<.3
 assert not list(work.glob('.omniget-clip-*')),'completed workspace was not cleaned'
 print('PASS: SIGKILL during real eight-track normalization; source checkpoint reused with unavailable original input; eight tracks and 60s final output validated')
