"""Bind four manual LOC/source reviews to the immutable completed full studies.

Use the fixture-qualified LOC18 adapter, with an additional check that it
cannot overwrite a contradictory raw environment_attempt. All pin checks and
record reconciliation hold the common timing mutex. No benchmark is invoked.
"""
import argparse
from contextlib import ExitStack
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
JOB = CLBG / 'loc-normalization/20261012/jobs/boundary-audit-18'
REVIEW = HERE / 'pending-minima-source-review-v1.json'
LOCK = Path('/tmp/mech-clbg-timing.lock')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def reconcile():
    receipt_path = JOB / 'fixture-receipt.json'
    receipt = read(receipt_path)
    require(receipt['status'] == 'passed' and type(receipt['tests_run']) is int and
            receipt['tests_run'] == 672 and receipt['failures'] == receipt['errors'] == 0 and
            receipt['exclusive_mutex_held'] is True, 'LOC18 fixtures are incomplete')
    for filename, digest in receipt['frozen_dependency_pins'].items():
        require(sha(filename) == digest, 'Frozen LOC18 dependency changed: ' + filename)
    module_path = JOB / 'snapshot/pending_minima_records.py'
    require(str(module_path) in receipt['frozen_dependency_pins'], 'Missing qualified adapter pin')
    spec = importlib.util.spec_from_file_location('qualified_pending_minima_adapter', module_path)
    adapter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(adapter)
    review = read(REVIEW)
    require(set(review['sources']) == set(adapter.STUDIES), 'Manual review inventory differs')
    audit_path = JOB / 'candidate-audit.json'
    audit = read(audit_path)
    require(audit['status'] == 'candidate-audit-awaiting-review' and
            audit['execution']['exclusive_mutex_held'] is True, 'Unexpected LOC18 evidence')
    checked, provenance = {}, {str(receipt_path): sha(receipt_path), str(REVIEW): sha(REVIEW),
        str(audit_path): sha(audit_path), str(module_path): sha(module_path), str(Path(__file__).resolve()): sha(__file__)}
    with ExitStack() as stack:
        for benchmark in adapter.STUDIES:
            base = CLBG / adapter.COHORT / benchmark / 'full-study'
            lease_path = base / 'owner.lock'
            require(lease_path.is_file(), 'Original study owner lease is missing')
            lease = stack.enter_context(lease_path.open('a+'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        for benchmark, study in adapter.STUDIES.items():
            base = CLBG / adapter.COHORT / benchmark / 'full-study'
            manifest_path, report_path = base / 'manifest.json', base / 'report.json'
            manifest, report = read(manifest_path), read(report_path)
            report_digest = sha(report_path)
            for number, attempt in enumerate(report['attempts'], 1):
                raw_path = Path(attempt['result_path']).resolve(strict=True)
                require(raw_path == base / 'attempts' / f'{number:03d}-{attempt["id"]}.json',
                        'Raw evidence escaped its actual study inventory')
                raw = read(raw_path)
                if 'environment_attempt' in raw:
                    require(type(raw['environment_attempt']) is int and
                            raw['environment_attempt'] == attempt['environment_attempt'],
                            'Contradictory original environment attempt')
                provenance[str(raw_path)] = sha(raw_path)
            rows, discarded = adapter.validate_study(manifest, report,
                manifest_sha256=sha(manifest_path), benchmark=benchmark, digest=sha)
            manual = review['sources'][benchmark]
            row = rows[study['identity']]
            require(row['source_sha256'] == manual['sha256'], 'Manual review names another source')
            candidates = [c for c in audit['candidates'] if c['key'] == manual['key']]
            require(len(candidates) == 1, 'Missing/duplicate LOC18 selected source')
            candidate = candidates[0]
            require(candidate['source_identities'] == [manual['sha256'] + ':' + study['language']],
                    'LOC18 source identity differs')
            source = audit['sources'][candidate['source_identities'][0]]
            require(source['diagnostics'] == candidate['diagnostics'] == [] and
                    source['physical_loc'] == manual['physical_loc'] and
                    source['normalized_loc'] == candidate['normalized_loc'] == manual['normalized_loc'],
                    'Manual count differs from qualified source normalization')
            expansions = {str(item['line']): item['extra_lines'] for item in source['expansions']}
            require(expansions == manual['extra_units_by_line'] and
                    manual['physical_loc'] + sum(expansions.values()) == manual['normalized_loc'],
                    'Manual expansion owners differ from saved source evidence')
            require(candidate['samples'] == row['samples_seconds'] and
                    candidate['seconds'] == row['median_seconds'], 'LOC18 timing association differs')
            require(sha(report_path) == report_digest, 'Completed report changed during review')
            checked[benchmark] = dict(key=manual['key'], source=row['source'],
                source_sha256=manual['sha256'], physical_loc=manual['physical_loc'],
                normalized_loc=manual['normalized_loc'], count_reason=manual['count_reason'],
                required_work_review=manual['required_work_review'],
                samples_seconds=row['samples_seconds'], median_seconds=row['median_seconds'],
                fresh_mech_control=rows['mech-aot-control'],
                discarded_environment_records=len(discarded), manifest=str(manifest_path),
                manifest_sha256=study['manifest_sha256'], report=str(report_path), report_sha256=report_digest)
            provenance.update({str(manifest_path): sha(manifest_path), str(report_path): report_digest,
                               row['source']: manual['sha256']})
    return dict(status='four-compact-sources-reviewed-and-reconciled-awaiting-publication',
        reviewed_sources=checked, actual_full_processes=24, source_samples=3,
        provenance=provenance, common_timing_mutex=str(LOCK), exclusive_mutex_held=True,
        chart_or_site_modified=False, benchmark_source_bytes_modified=False,
        measurements_rewritten=False, completed_at=datetime.now(timezone.utc).isoformat())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    require(output.is_relative_to(HERE) and output.suffix == '.json', 'Output must be a new continuation JSON')
    print('WAIT common mutex: reconcile four completed compact source reviews', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        require(not output.exists(), 'Never overwrite a reviewed record')
        result = reconcile()
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open('x') as stream:
            json.dump(result, stream, indent=2)
            stream.write('\n')
        print(json.dumps(dict(output=str(output), status=result['status'])), flush=True)


if __name__ == '__main__':
    main()
