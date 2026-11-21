"""Run after cargo build --manifest-path .github/android/clip-tests/Cargo.toml."""
from pathlib import Path
import subprocess, tempfile, json, shutil, concurrent.futures, struct
BIN=Path(__file__).parent.resolve()/'target/debug/clip-harness'
def run(*args): return subprocess.check_output(list(map(str,args)),text=True,stderr=subprocess.PIPE)
def probe(path): return json.loads(run('ffprobe','-v','error','-show_format','-show_streams','-of','json',path))
with tempfile.TemporaryDirectory() as temp:
    root=Path(temp)
    # Simulate the real complaint: a section containing 60s of media carries
    # timestamps from the original long video and reports over 30 minutes.
    source=root/'shifted.mkv'
    run('ffmpeg','-v','error','-f','lavfi','-i','color=size=160x90:rate=12','-f','lavfi','-i','sine=frequency=440','-t','60','-c:v','libx264','-c:a','aac','-output_ts_offset','1800',source)
    # Force the Matroska Duration field to 31 minutes. Newer FFmpeg builds
    # repair MP4 edit-list offsets automatically, so use an explicit corrupt
    # duration field rather than depending on the muxer's version.
    data=bytearray(source.read_bytes())
    marker=data.index(bytes([0x44,0x89,0x88]))
    struct.pack_into('>d',data,marker+3,1_860_000.0)
    source.write_bytes(data)
    assert float(probe(source)['format']['duration'])>1800
    destination=root/'clips'
    def clip(n):
        staging=root/f'job{n}';staging.mkdir();input=staging/'source.mkv';shutil.copyfile(source,input)
        return Path(run(BIN,input,destination,'0:30-1:30','1800').splitlines()[0])
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: paths=list(pool.map(clip,range(3)))
    assert len(set(paths))==3
    assert {p.name for p in paths}=={'Test title (from 0:30 to 1:30).mp4','Test title (from 0:30 to 1:30) (1).mp4','Test title (from 0:30 to 1:30) (2).mp4'}
    for path in paths:
        data=probe(path);assert abs(float(data['format']['duration'])-60)<0.3
        assert all(abs(float(s['start_time']))<0.1 for s in data['streams'])
        assert all(abs(float(s['duration'])-60)<0.3 for s in data['streams'])
    # A requested end after EOF is clamped to the real available duration.
    short=root/'short.mp4';run('ffmpeg','-v','error','-i',paths[0],'-t','5','-c','copy',short)
    eof=Path(run(BIN,short,destination,'0:10-0:30','15').splitlines()[0])
    assert abs(float(probe(eof)['format']['duration'])-5)<0.3
    # Audio-only clips stay audio-only and keep their requested MP3 format.
    audio=root/'source.mp3';run('ffmpeg','-v','error','-i',paths[0],'-t','4','-vn','-c:a','libmp3lame',audio)
    mp3=Path(run(BIN,audio,destination,'0:00-0:04','60').splitlines()[0])
    assert mp3.suffix=='.mp3';assert [s['codec_type'] for s in probe(mp3)['streams']]==['audio']
    assert abs(float(probe(mp3)['format']['duration'])-4)<0.3
    # Device regression: an AV1 section can decode slightly short of its
    # integer-valued requested span. Keep actual duration, never pad/fake it.
    av1=root/'av1-short.mp4'
    run('ffmpeg','-v','error','-f','lavfi','-i','testsrc2=size=160x90:rate=24000/1001','-f','lavfi','-i','sine=frequency=440:sample_rate=44100','-t','11.652','-c:v','libaom-av1','-cpu-used','8','-crf','40','-c:a','aac',av1)
    fixed=Path(run(BIN,av1,destination,'0:00-0:12','15').splitlines()[0])
    measured=probe(fixed)
    assert 11.5<float(measured['format']['duration'])<12
    assert next(s for s in measured['streams'] if s['codec_type']=='video')['codec_name']=='h264'
    assert all(abs(float(s['start_time']))<0.1 for s in measured['streams'])
    # A one-second loss is a real truncation and must still fail validation.
    broken=root/'broken.mp4';run('ffmpeg','-v','error','-i',av1,'-t','11','-c','copy',broken)
    rejected=subprocess.run([str(BIN),str(broken),str(destination),'0:00-0:12','15'],capture_output=True,text=True)
    assert rejected.returncode!=0 and 'Clip duration mismatch' in rejected.stderr
    print('PASS: timestamp phantom; concurrent names; EOF; MP3; short AV1 accepted; real truncation rejected')
