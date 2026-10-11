"""Revise only unmeasured nucleotide rows while preserving every saved sample."""
import copy, hashlib, json
from datetime import datetime, timezone
from pathlib import Path

D = Path(__file__).resolve().parent
A = D / 'evidence/dictionary-before-indexing'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    assert (D / 'pause-measurements').exists()
    manifest = json.loads((A / 'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        assert sha(A / name) == digest, name
    path = D / 'results/timings.json'
    assert sha(path) == manifest['files']['results/timings.json']
    original = json.loads(path.read_text())
    report = copy.deepcopy(original)
    built = json.loads((D / 'results/builds.json').read_text())
    audit = json.loads((D / 'source-audit.json').read_text())
    small = json.loads((D / 'results/small-checks.json').read_text())
    assert audit['status'] == 'reviewed' and small['status'] == 'passed'
    assert audit['builds_sha256'] == sha(D / 'results/builds.json')
    assert audit['small_checks_sha256'] == sha(D / 'results/small-checks.json')
    assert report['runtimes'] == built['runtimes']
    changed = []
    for prepared in built['programs']:
        key = prepared['key']
        if prepared['benchmark'] != 'knucleotide':
            assert prepared['source_sha256'] == original['programs'][key]['source_sha256']
            continue
        before = original['programs'][key]
        assert before['samples_seconds'] == before['sample_validation'] == []
        assert small['programs'][key]['source_sha256'] == prepared['source_sha256']
        assert small['programs'][key]['status'] == 'passed'
        assert sha(prepared['source']) == prepared['source_sha256']
        row = copy.deepcopy(prepared)
        loc = row['loc_audit']
        row.update(loc=loc['normalized_loc'], physical_loc=loc['physical_loc'],
                   loc_expansions=loc['expansions'], algorithm=audit['requirements']['knucleotide'],
                   small_validation=small['programs'][key]['validation'], samples_seconds=[],
                   sample_validation=[], eligible_for_chart=True)
        report['programs'][key] = row
        changed.append(dict(key=key, previous_source_sha256=before['source_sha256'],
                            source_sha256=row['source_sha256'], previous_loc=before['loc'], loc=row['loc'],
                            previous_full_samples=0))
    assert len(changed) == 2
    changed_keys = {row['key'] for row in changed}
    preserved = {key: row for key, row in original['programs'].items() if key not in changed_keys}
    for key, row in preserved.items():
        assert row == report['programs'][key]
        assert sha(row['source']) == row['source_sha256']
    samples = sum(len(row['samples_seconds']) for row in report['programs'].values())
    assert samples == manifest['preserved_samples'] == 69
    report['source_revisions'] = report.get('source_revisions', []) + [dict(
        at=datetime.now(timezone.utc).isoformat(), reason=manifest['reason'], changed=changed,
        archive=str(A), archive_manifest_sha256=sha(A / 'manifest.json'),
        previous_report_sha256=manifest['files']['results/timings.json'], preserved_samples=samples,
        preserved_rows_sha256=hashlib.sha256(json.dumps(preserved, sort_keys=True).encode()).hexdigest(),
        diagnostic=str(D / 'results/dictionary-probe.json'), diagnostic_sha256=sha(D / 'results/dictionary-probe.json'))]
    report['source_pins'] = {name: sha(D / name) for name in original['source_pins']}
    report['source_pins']['rebase_pending.py'] = sha(Path(__file__))
    report['protocol'] += (' Before any full nucleotide measurements, its two sources were revised to use ordinary '
                           'dictionary indexing with exactly represented double integer keys. Compact initialization '
                           'adds a complete zero-insertion pass; both sources retain all 874999961 explicit counter '
                           'updates. The generic API diagnostic is untimed evidence, never a chart sample. '
                           'The original sources, evidence and report are archived; all 69 earlier samples remain unchanged.')
    temporary = path.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(report, indent=2) + '\n')
    temporary.replace(path)
    revision_path = D / 'results/dictionary-revision.json'
    revision = json.loads(revision_path.read_text())
    revision.update(status='prepared-for-resume', changed=changed, preserved_samples=samples,
                    report_sha256=sha(path), revised_at=datetime.now(timezone.utc).isoformat())
    revision_path.write_text(json.dumps(revision, indent=2) + '\n')
    print('Preserved all', samples, 'samples; revised only the two unmeasured nucleotide rows.')
    for row in changed:
        print(row['key'], row['loc'], 'normalized LOC')

if __name__ == '__main__':
    main()
