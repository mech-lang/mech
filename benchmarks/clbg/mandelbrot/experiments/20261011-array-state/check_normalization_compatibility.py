#!/usr/bin/env python3
"""Compare old/current normalization of existing benchmark sources, not timings."""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
RESULTS = HERE / 'normalization-compatibility'
APP = ROOT / 'src/engine/src/native_sequence/application.rs'


def sha(data):
    return hashlib.sha256(data).hexdigest()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--before-ref',
                    help='Prior Git revision; defaults to the saved compatibility report, or HEAD for a first run.')
args = parser.parse_args()

with open('/tmp/mech-clbg-timing.lock', 'a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    RESULTS.mkdir(exist_ok=True)
    saved_report = RESULTS / 'report.json'
    before_ref = args.before_ref
    if before_ref is None:
        before_ref = json.loads(saved_report.read_text())['prior_revision'] if saved_report.exists() else 'HEAD'
    revision = subprocess.check_output(['git', 'rev-parse', before_ref], cwd=ROOT, text=True).strip()
    prior = subprocess.check_output(['git', 'show', revision + ':src/engine/src/native_sequence/application.rs'], cwd=ROOT)
    current = APP.read_bytes()
    sources = sorted(path for path in (ROOT / 'benchmarks/clbg').rglob('*.mec')
                     if '.build' not in path.parts and HERE not in path.parents)
    rust_source = r'''
#[path = "before.rs"] mod before;
#[path = "after.rs"] mod after;
use std::io::{self, BufRead};
fn main() {
    for path in io::stdin().lock().lines() {
        let path = path.unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let old = before::normalize(&source);
        let new = after::normalize(&source);
        let label = match (&old, &new) {
            (Ok(a), Ok(b)) if a == b => "unchanged-ok",
            (Err(a), Err(b)) if a == b => "unchanged-error",
            (Err(_), Ok(_)) => "newly-accepted",
            (Ok(_), Err(_)) => "new-error",
            (Ok(_), Ok(_)) => "changed-normalization",
            (Err(_), Err(_)) => "changed-error",
        };
        println!("{}\t{}", label, path);
    }
}
'''
    with tempfile.TemporaryDirectory(prefix='mech-normalization-compatibility-') as directory:
        build = Path(directory)
        (build / 'before.rs').write_bytes(prior)
        (build / 'after.rs').write_bytes(current)
        (build / 'main.rs').write_text(rust_source)
        rustc = '/Users/cim310/.cargo/bin/rustc'
        compiled = subprocess.run([rustc, '--edition=2024', str(build / 'main.rs'), '-o', str(build / 'compare')], capture_output=True, text=True)
        (RESULTS / 'build.log').write_text(compiled.stdout + compiled.stderr)
        compiled.check_returncode()
        compared = subprocess.run([str(build / 'compare')], input=''.join(str(path) + '\n' for path in sources), capture_output=True, text=True, check=True)
        rows = []
        for line in compared.stdout.splitlines():
            status, path = line.split('\t', 1)
            rows.append({'source': str(Path(path).relative_to(ROOT)), 'source_sha256': sha(Path(path).read_bytes()), 'status': status})
        counts = {status: sum(row['status'] == status for row in rows) for status in sorted(set(row['status'] for row in rows))}
        report = {'scope': 'Existing benchmark sources excluding new Mandelbrot experiment and ignored build directories; normalization only, no timing or native execution claim.',
                  'prior_revision': revision, 'prior_application_sha256': sha(prior), 'current_application_sha256': sha(current),
                  'rustc_version': subprocess.check_output([rustc, '-Vv'], text=True), 'sources': rows, 'counts': counts}
        (RESULTS / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps({'sources': len(rows), 'counts': counts}), flush=True)
        if counts.get('new-error'):
            raise SystemExit('Existing accepted benchmark normalization regressed; inspect report.')
