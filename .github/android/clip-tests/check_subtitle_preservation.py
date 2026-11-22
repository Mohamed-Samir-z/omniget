"""Offline regression: invalid/missing captions must not discard good media."""
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile

BIN = Path(__file__).parent.resolve() / 'target/debug/clip-harness'
def run(*args):
    return subprocess.check_output(list(map(str, args)), text=True, stderr=subprocess.PIPE)
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    fixture = root / 'fixture.mp4'
    run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=size=160x90:rate=12', '-t', '4', '-c:v', 'libx264', fixture)
    for name, captions in [('missing', None), ('empty', b''), ('html', b'<html>Unavailable</html>'), ('invalid-utf8', bytes([255, 254])), ('valid', b'1\n00:00:10,500 --> 00:00:12,000\nHello\n')]:
        staging = root / name
        staging.mkdir()
        source = staging / 'source.mp4'
        source.write_bytes(fixture.read_bytes())
        before = hashlib.sha256(source.read_bytes()).hexdigest()
        if captions is not None:
            (staging / 'source.ar.srt').write_bytes(captions)
        output = Path(run(BIN, source, root / 'finished', '0:10-0:14', '30').splitlines()[0])
        assert output.is_file() and output.stat().st_size > 0
        info = json.loads(run('ffprobe', '-v', 'error', '-show_format', '-of', 'json', output))
        assert abs(float(info['format']['duration']) - 4) < .3
        assert hashlib.sha256(source.read_bytes()).hexdigest() == before
        sidecar = output.with_name(output.stem + '.ar.srt')
        if name == 'valid':
            assert '00:00:00,500 --> 00:00:02,000' in sidecar.read_text()
        else:
            assert not sidecar.exists(), name
    print('PASS: missing, empty, HTML and invalid-UTF8 captions preserve valid video and original source; valid SRT is trimmed/rebased')
