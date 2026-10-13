"""Review the sealed Clojure suite and durably queue its ten original owners."""
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
COHORT = HERE.parent / 'clojure-comparison/20261012'
INDEX = COHORT / 'studies/full-v1/index.json'
INDEX_SHA = '7391f432d86a78b0995a1895ff828930642c2eb82f4dbcb36ef86fe97c75b6fd'
CONTROLLER = COHORT / 'revisions/work-full-v1/full_study.py'
BENCHMARKS = ('regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    assert sha(INDEX) == INDEX_SHA
    index = json.loads(INDEX.read_text())
    assert index['status'] == 'sealed-awaiting-root-review'
    assert index['samples'] == 3 and index['full_processes'] == 90
    assert set(index['benchmarks']) == set(BENCHMARKS)
    manifests = {}
    for benchmark in BENCHMARKS:
        entry = index['benchmarks'][benchmark]
        path = Path(entry['manifest'])
        assert path == COHORT / 'studies/full-v1' / benchmark / 'manifest.json'
        assert sha(path) == entry['sha256']
        manifest = json.loads(path.read_text())
        assert manifest['pins'][str(CONTROLLER)] == sha(CONTROLLER)
        assert not (path.parent / 'report.json').exists()
        manifests[benchmark] = manifest
    spec = importlib.util.spec_from_file_location('clojure_reviewed_full90_entry', CONTROLLER)
    controller = importlib.util.module_from_spec(spec)
    exec(compile(CONTROLLER.read_bytes(), str(CONTROLLER), 'exec'), controller.__dict__)
    assert tuple(controller.BENCHMARKS) == BENCHMARKS
    for benchmark in BENCHMARKS:
        output = COHORT / 'studies/full-v1' / benchmark
        guard = controller.configure(output)
        controller.review(guard, benchmark, 'full-v1', index['benchmarks'][benchmark]['sha256'])
    output = HERE / 'jobs/clojure-full90-launch-v1'
    output.mkdir(parents=True)
    launches = []
    for benchmark in BENCHMARKS:
        command = [sys.executable, '-B', '-u', str(CONTROLLER), '--run', '--benchmark',
                   benchmark, '--revision', 'full-v1', '--manifest-sha256',
                   index['benchmarks'][benchmark]['sha256']]
        result = subprocess.run([sys.executable, '-B', str(HERE / 'launch_durable_v1.py'),
            '--job', 'clojure-full-v1-' + benchmark, '--cwd', str(ROOT),
            '--command-json', json.dumps(command)], capture_output=True, text=True, check=True)
        receipt = json.loads(result.stdout)
        launches.append(receipt)
        with (output / (benchmark + '.json')).open('x') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
    with (output / 'receipt.json').open('x') as stream:
        json.dump(dict(status='ten-reviewed-study-owners-durably-queued',
                       index_sha256=INDEX_SHA, launches=launches,
                       planned_full_processes=90, chart_admission=False), stream, indent=2)
        stream.write('\n')
    print(json.dumps(dict(status='ten-reviewed-study-owners-durably-queued', owners=10)), flush=True)
