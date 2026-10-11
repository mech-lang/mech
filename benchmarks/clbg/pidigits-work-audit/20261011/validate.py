"""Small byte-for-byte checks only; every process holds the shared timing lock."""
import datetime
import fcntl
import hashlib
import json
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]


def digest(value):
    return hashlib.sha256(value).hexdigest()


def execute(command):
    p = subprocess.run(command, cwd=CLBG, capture_output=True, timeout=30)
    return p, dict(command=command, returncode=p.returncode,
        stdout=p.stdout.decode(), stdout_bytes=len(p.stdout), stdout_sha256=digest(p.stdout),
        stderr=p.stderr.decode(), stderr_sha256=digest(p.stderr))


def main():
    audit_path = HERE / 'audit.json'
    audit = json.loads(audit_path.read_text())
    row = audit['corrections'][0]
    node = row['runtime']['path']
    assert digest(Path(node).read_bytes()) == row['runtime']['sha256']
    assert digest(Path(row['source']).read_bytes()) == row['source_sha256']
    assert digest(Path(row['upstream_source']).read_bytes()) == row['upstream_sha256']
    manifest = json.loads((CLBG / 'd-comparison/20261011/tree-digit-manifest.json').read_text())
    reference = next(r for r in manifest['programs'] if r['id'] == 'pidigits:d-fast')
    assert digest(Path(reference['binary']).read_bytes()) == reference['binary_sha256']
    cases = []
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        deadline = time.monotonic() + 50
        while True:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() > deadline:
                    print('Shared lock busy; no benchmark process executed.')
                    return
                time.sleep(0.2)
        for n in (1, 9, 10, 27, 30, 101):
            corrected, detail = execute([node, row['source'], str(n)])
            upstream, prior = execute([node, row['upstream_source'], str(n)])
            independent, expected = execute([reference['binary'], str(n)])
            passed = all(p.returncode == 0 and not p.stderr for p in (corrected, upstream, independent)) and corrected.stdout == upstream.stdout == independent.stdout
            if n == 30:
                passed = passed and corrected.stdout == b'3141592653\t:10\n5897932384\t:20\n6264338327\t:30\n'
            cases.append(dict(n=n, passed=passed, corrected=detail, upstream=prior, independent_reference=expected))
        fcntl.flock(lock, fcntl.LOCK_UN)
    report = dict(schema_version=1, created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        purpose='Small correctness checks, not performance measurements.',
        lock='/tmp/mech-clbg-timing.lock', source_sha256=row['source_sha256'],
        upstream_sha256=row['upstream_sha256'], runtime=row['runtime'],
        reference=dict(source=reference['source'], source_sha256=reference['source_sha256'],
                       binary=reference['binary'], binary_sha256=reference['binary_sha256']),
        cases=cases, passed=all(r['passed'] for r in cases))
    report_path = HERE / 'results/small-validation.json'
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed'], 'At least one small validation failed; full timing is not admitted.'
    row['validation_status'] = 'six_small_byte_exact_checks_passed'
    row['small_validation'] = dict(report=str(report_path), sha256=digest(report_path.read_bytes()),
                                   n=[r['n'] for r in cases], passed=True)
    audit['status'] = 'corrected_source_validated_pending_full_timing'
    audit_path.write_text(json.dumps(audit, indent=2) + '\n')
    print(json.dumps(dict(passed=report['passed'], cases=len(cases), source_sha256=row['source_sha256']), indent=2))


if __name__ == '__main__':
    main()
