"""Reconcile all ten V workloads, retaining the additive tree continuation."""
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
OUT = HERE / 'reconciliation/all-ten-full-samples-v3.json'
TREE = HERE / 'studies/binarytrees/remaining-eight-v1/report.json'
TREE_SHA = 'c24c7f84243c15e28e78dbe0ce2bd40282ba35079931dbb58df69130d19a4b79'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    assert not OUT.exists()
    prior = json.loads((HERE / 'reconciliation/regexredux-three-full-samples-v2.json').read_text())
    source = HERE / 'reconcile_full_v2.py'
    assert sha(source) == prior['reconciler_sha256']
    sys.path.insert(0, str(HERE))
    spec = importlib.util.spec_from_file_location('v_exact_saved_records_v2', source)
    validator = importlib.util.module_from_spec(spec)
    exec(compile(source.read_bytes(), str(source), 'exec'), validator.__dict__)
    pins = {str(source): sha(source), str(Path(__file__).resolve()): sha(__file__)}
    results = {}
    for benchmark in validator.full_study.BENCHMARKS:
        module = validator.full_study.configure(benchmark)
        manifest_path = module.MANIFEST
        manifest = json.loads(manifest_path.read_text())
        digest = sha(manifest_path)
        validator.full_study.review_manifest(module, digest, benchmark)
        harness = module.load_harness()
        report_path = TREE if benchmark == 'binarytrees' else module.REPORT
        report = json.loads(report_path.read_text())
        if benchmark == 'binarytrees':
            assert sha(report_path) == TREE_SHA
            assert report['retained_attempt_count'] == 1 and report['fresh_attempt_count'] == 8
            assert report['original_report_mutated'] is False
            module.verify(report['pins'])
            pins.update(report['pins'])
            roots = ((module.OUT / 'attempts').resolve(), (TREE.parent / 'attempts').resolve())
        else:
            roots = ((module.OUT / 'attempts').resolve(),)
        assert Path(report['manifest']).resolve() == manifest_path
        def raw(name):
            path = Path(name).resolve(strict=True)
            assert any(path.is_relative_to(root) for root in roots)
            pins[str(path)] = sha(path)
            return json.loads(path.read_text())
        rows, exclusions = validator.validate_data(manifest, report, digest, harness,
            validator.expected_output(harness, benchmark), raw)
        control = next(row for row in rows.values() if row['language'] == 'mech')
        for row in rows.values():
            row['time_ratio_to_matched_mech'] = row['median_seconds'] / control['median_seconds']
        results[benchmark] = dict(status='reconciled-three-full-samples', profiles=rows,
            environment_exclusions=exclusions, manifest=str(manifest_path), manifest_sha256=digest,
            report=str(report_path), report_sha256=sha(report_path))
        pins.update(manifest['pins'])
        pins[str(manifest_path)] = digest
        pins[str(report_path)] = sha(report_path)
    assert len(results) == 10
    for name, digest in pins.items():
        assert sha(name) == digest
    result = dict(status='all-ten-workloads-independently-reconciled', benchmarks=results,
        canonical_full_native_processes=90, candidate_sources=20, actual_samples_per_source=3,
        report_only=True, never_chart_eligible=True, chart_admission=False,
        original_reports_mutated=False, pins=pins)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open('x') as stream:
        json.dump(result, stream, indent=2)
        stream.write('\n')
    for benchmark, entry in results.items():
        best = min((row for row in entry['profiles'].values() if row['language'] == 'v'),
                   key=lambda row: row['median_seconds'])
        print(benchmark, best['variant'], f"{best['time_ratio_to_matched_mech']:.3f} x Mech", flush=True)
