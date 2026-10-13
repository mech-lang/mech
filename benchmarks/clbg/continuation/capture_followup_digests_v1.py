"""Capture immutable completed inputs for the next reviewed stages."""
import fcntl
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
PATHS = (
    'pony-comparison/20261012/work-qualification/private-v8-work-v3/report.json',
    'nim-comparison/20261012/work-qualification/work-v5/report.json',
    'gleam-comparison/20261012/work-qualification/work-v4/report.json',
    'prolog-comparison/20261013/proposals/private-v1.json',
    'v-report-only/20261012/studies/binarytrees/remaining-eight-v1/report.json',
    'matlab-comparison/20261012/regex-case-sensitive-v1/studies/full-v1/regexredux/report.json',
)
with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    output = HERE / 'jobs/followup-digests-v1'
    output.mkdir(parents=True)
    pins = {}
    for name in PATHS:
        path = CLBG / name
        with path.open('rb') as stream:
            pins[str(path)] = hashlib.file_digest(stream, 'sha256').hexdigest()
    with (output / 'receipt.json').open('x') as stream:
        json.dump(dict(status='completed-input-digests-captured', pins=pins,
                       common_timing_mutex_held=True, benchmark_runs=0), stream, indent=2)
        stream.write('\n')
    print(json.dumps(pins), flush=True)
