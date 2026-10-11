"""Repair published regex port startup flags while retaining all completed samples."""
import json
from datetime import datetime, timezone
from pathlib import Path
from prepare import sha
import audit
import run as runner

D = Path(__file__).resolve().parent


def main():
    paths = ['results/builds.json', 'results/small-checks.json', 'source-audit.json', 'results/timings.json']
    snapshots = {}
    for relative in paths:
        source = D / relative
        destination = source.with_name('pre-regex-runtime-fix-' + source.name)
        assert not destination.exists(), destination
        destination.write_bytes(source.read_bytes())
        snapshots[relative] = dict(file=str(destination.relative_to(D)), sha256=sha(destination))
    builds_path = D / 'results/builds.json'
    built = json.loads(builds_path.read_text())
    changes = []
    for row in built['programs']:
        if row['benchmark'] != 'regexredux':
            continue
        before = row['command'].copy()
        row['command'].insert(row['command'].index('-noshell') + 1, '-noinput')
        changes.append(dict(key=row['key'], before=before, after=row['command'], binary_sha256=row['binary_sha256'], source_sha256=row['source_sha256']))
    assert len(changes) == 2
    runner.save(builds_path, built)
    small_path = D / 'results/small-checks.json'
    small = json.loads(small_path.read_text())
    for row in built['programs']:
        if row['benchmark'] != 'regexredux':
            continue
        result = runner.execute(row, False, 120)
        assert not result['competing_compilers'] and not result['power_changes']
        small['programs'][row['key']] = dict(status='passed', source_sha256=row['source_sha256'], **result)
    runner.save(small_path, small)
    audit.main()
    report_path = D / 'results/timings.json'
    report = json.loads(report_path.read_text())
    for change in changes:
        row = report['programs'].get(change['key'])
        if row:
            assert not row['samples_seconds']
            row['command'] = change['after']
            row['small_validation'] = small['programs'][row['key']]['validation']
    evidence = D / 'results/regexredux-full-failure.txt'
    amendment = dict(created_at=datetime.now(timezone.utc).isoformat(), status='repaired', snapshots=snapshots, changes=changes,
        preserved_samples=sum(len(r['samples_seconds']) for r in report['programs'].values()),
        reason='The original published regex-redux launch uses -noinput with its direct fd port. OTP 29 default standard input initialization also claimed fd 0 and intermittently emitted a driver_select warning into stdout. Restore the published startup flag for regex only; source, BEAM binaries, workload and every already completed sample are unchanged.',
        rejected_full_attempt=dict(benchmark='regexredux', reason='Startup diagnostic polluted complete stdout; sample was never admitted.',
            diagnostic_file=str(evidence.relative_to(D)), diagnostic_sha256=sha(evidence), timed=False))
    amendment_path = D / 'runtime-amendment.json'
    runner.save(amendment_path, amendment)
    report['source_pins'].update({relative: sha(D / relative) for relative in ['build.py', 'source-audit.json', 'results/builds.json', 'results/small-checks.json', 'repair_runtime.py', 'runtime-amendment.json']})
    report['runtime_amendments'] = [dict(file='runtime-amendment.json', sha256=sha(amendment_path), preserved_samples=amendment['preserved_samples'])]
    runner.save(report_path, report)
    print('Restored published regex-redux -noinput; preserved', amendment['preserved_samples'], 'validated samples')


if __name__ == '__main__':
    main()
