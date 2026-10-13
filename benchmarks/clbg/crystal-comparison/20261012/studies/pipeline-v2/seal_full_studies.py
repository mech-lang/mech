"""Seal actual qualified Crystal sources and full workloads, without execution."""
import argparse
import copy
import fcntl
import importlib.util
import json
from pathlib import Path
import sys

import full_study

HERE = full_study.HERE
GATES = full_study.GATES


def load_preparation():
    path = HERE / 'guarded_prepare_v6.py'
    spec = importlib.util.spec_from_file_location('crystal_full_preparation', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def exact_true_checks(attempt):
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is False
    assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert attempt['validation']['reference_match'] is True
    assert attempt['pins_unchanged_at_end'] is True
    assert attempt['full_performance_run'] is False


def validate_preparation(preparation, guard, report_path):
    qualified = json.loads(report_path.read_text())
    assert qualified['status'] == 'bounded-output-ownership-work-passed-awaiting-LOC-and-full-review'
    assert qualified['pins_unchanged_at_end'] is True and qualified['active'] is None
    assert qualified['full_performance_runs'] == qualified['prior_cases_reexecuted'] == 0
    assert qualified['reused_native_cases'] == 112 and qualified['fresh_bounded_output_cases'] == 44
    assert qualified['fresh_profile_builds'] == 5 and qualified['full_manifests_sealed'] == 0
    assert qualified['prior_records_relabelled_as_new_attempts'] is False
    guard.verify(qualified['pins'])
    rows = qualified['profiles']
    assert len(rows) == 20 and len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {
        (benchmark, variant) for benchmark in full_study.BENCHMARKS for variant in ('compact', 'optimized')}
    attempts = qualified['attempts']
    assert len({attempt['attempt'] for attempt in attempts}) == len(attempts)
    for attempt in attempts:
        exact_true_checks(attempt)
    reused = Path(qualified['exact_prior_proof'])
    assert preparation.sha(reused) == qualified['exact_prior_proof_sha256']
    reuse = json.loads(reused.read_text())
    assert reuse['status'] == 'verified-exact-old-bounded-proof'
    assert reuse['prior_bounded_cases'] == 112 and reuse['native_cases_reexecuted'] == 0
    guard.verify(reuse['pins'])
    reference = preparation.load_module('crystal_full_case_inventory', preparation.V / 'references.py')
    inventories = preparation.preflight_inputs(reference)
    bounded = [attempt for attempt in attempts if attempt['measurement_kind'] == 'bounded-output']
    assert len(bounded) == 44
    for row in rows:
        original = HERE / 'sources' / row['benchmark'] / (row['variant'] + '.cr')
        assert preparation.sha(original) == preparation.sha(row['source']) == row['source_sha256']
        assert preparation.sha(row['binary']) == row['binary_sha256']
        assert row['maximum_workers'] <= 8
        fresh = [attempt for attempt in bounded if attempt['id'] == row['id']]
        prior = reuse['profiles'].get(row['id'], {}).get('prior_records', [])
        names = [attempt['case'] for attempt in fresh] + [item['case'] for item in prior]
        expected = {'n-' + str(n) for n in preparation.NUMERIC[row['benchmark']]} if row['benchmark'] in preparation.NUMERIC else {
            name for name, _ in inventories[row['benchmark']]}
        assert len(names) == len(set(names)) and set(names) == expected, ('Incomplete bounded profile', row['id'])
        for attempt in fresh:
            assert attempt['source_sha256'] == row['source_sha256']
            assert attempt['execution_fingerprint'] == guard.load_harness().execution_fingerprint(row)
    proofs = qualified['proofs']
    assert proofs['status'] == 'passed'
    primitive = [review for review in proofs['reviews'] if review.get('scope') == 'standard-primitives']
    assert len(primitive) == 1 and primitive[0]['status'] == 'passed'
    for field in ('floor_and_truncated_division', 'immutable_bigint_lifetime', 'long_string_constructor',
                  'complete_private_map_handoff', 'actual_capped_context', 'regex_nonoverlap_and_order', 'binary_nul_stdout'):
        assert primitive[0][field] is True
    reviews = [review for review in proofs['reviews'] if review.get('profile')]
    expected = set()
    selected = {row['id']: row for row in rows}
    for row in rows:
        if row['benchmark'] == 'knucleotide':
            names = {name for name, _ in inventories['knucleotide']}
        elif row['benchmark'] == 'pidigits':
            names = {'digits-97', 'zero-digits'}
        elif row['benchmark'] == 'binarytrees':
            names = {'allocated-n0', 'allocated-n7', 'allocated-n9'}
        else:
            continue
        expected |= {(row['id'], name) for name in names}
    observed = set()
    for review in reviews:
        key = (review['profile'], review['case'])
        assert key in expected and key not in observed
        observed.add(key)
        row = selected[review['profile']]
        assert review['status'] == 'passed' and review['measured_source_bytes_unchanged'] is True
        assert review['scored_source_sha256'] == row['source_sha256']
        attempt = next(attempt for attempt in attempts if attempt['attempt'] == review['attempt'])
        assert attempt['id'] == row['id'] + '-work-trace' and attempt['case'] == review['case']
        assert attempt['source_sha256'] == review['trace_source_sha256']
        assert attempt['source_work_proof']['status'] == 'passed'
    assert observed == expected
    return qualified, preparation.merge(qualified['pins'], reuse['pins'], {
        str(report_path): preparation.sha(report_path), str(reused): preparation.sha(reused)})


def full_reference(harness, benchmark, preparation):
    if benchmark == 'nbody':
        return None, []
    if benchmark in ('pidigits', 'regexredux'):
        path = full_study.CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=path.stat().st_size, sha256=preparation.sha(path)), [path]
    if benchmark in ('spectralnorm', 'fannkuchredux'):
        output = {'spectralnorm': b'1.274224153\n',
                  'fannkuchredux': b'3968050\nPfannkuchen(12) = 65\n'}[benchmark]
        import hashlib
        return dict(bytes=len(output), sha256=hashlib.sha256(output).hexdigest()), []
    size, digest = harness.compact.HASHES[benchmark][1]
    return dict(bytes=size, sha256=digest), []


def seal(args):
    print('WAIT common mutex: Crystal exact full-study manifests', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        preparation = load_preparation()
        guard = preparation.load_guard()
        harness = guard.load_harness()
        fixture_path = Path(__file__).with_name('full-study-fixtures.json')
        fixture = json.loads(fixture_path.read_text())
        assert fixture['status'] == 'passed' and fixture['tests_run'] == 7
        assert fixture['failures'] == fixture['errors'] == fixture['native_probes'] == fixture['actual_signals_sent'] == 0
        guard.verify(fixture['pins'])
        report_path = HERE / 'qualification' / args.qualified_revision / 'report.json'
        qualified, common = validate_preparation(preparation, guard, report_path)
        audit_path = full_study.SUITE / 'source-audit-r.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        evidence = [audit_path, Path(__file__).resolve(), Path(full_study.__file__).resolve(),
            Path(__file__).with_name('REQUIRED_WORK_REVIEW.md'), full_study.GUARD,
            full_study.SUITE / 'run.py', full_study.SUITE / 'prepare.py', fixture_path]
        common = preparation.merge(common, audit['pins'], fixture['pins'],
                                   {str(path): preparation.sha(path) for path in evidence})
        output = HERE / 'studies' / args.study_revision
        assert not output.exists(), 'Explicit new study revision; never replace earlier evidence'
        output.mkdir(parents=True)
        index = dict(status='sealed-awaiting-root-review', qualified_preparation=str(report_path),
                     qualified_preparation_sha256=preparation.sha(report_path), samples=3,
                     benchmarks={}, full_processes=90, chart_admission=False, no_automatic_retry=True)
        for benchmark in full_study.BENCHMARKS:
            profiles = [copy.deepcopy(row) for row in qualified['profiles'] if row['benchmark'] == benchmark]
            pins = dict(common)
            for row in profiles:
                row.update(loc=None, producer_loc=None, loc_status='pending separate immutable canonical LOC overlay',
                    eligible_for_chart=False, chart_admission=False, role='Qualified complete Crystal ' + row['variant'] + ' application',
                    required_work_review=dict(status='passed', method='Complete source review plus independent bounded outputs and actual primitive/map/division/node traces',
                        source_sha256=row['source_sha256'], review=str(Path(__file__).with_name('REQUIRED_WORK_REVIEW.md')),
                        qualification=str(report_path)))
            control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == benchmark))
            if benchmark == 'mandelbrot':
                path = full_study.CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
                study = json.loads(path.read_text())
                control = copy.deepcopy(next(row for row in study['profiles'] if row['id'] == 'row16-aot-control'))
                pins = preparation.merge(pins, study['pins'], {str(path): preparation.sha(path)})
            assert preparation.sha(control['source']) == control['source_sha256']
            assert preparation.sha(control['binary']) == control['binary_sha256']
            pins = preparation.merge(pins, control.get('support', {}), {
                str(Path(control['source'])): control['source_sha256'], str(Path(control['binary'])): control['binary_sha256']})
            control.update(id='mech-aot-control', key=benchmark + ':crystal-full-study:mech-control',
                producer_loc=control.get('loc'), loc=None, loc_status='pending canonical count overlay', maximum_workers=8,
                role='Fresh unchanged current fastest Mech AOT source control')
            profiles.append(control)
            reference, reference_paths = full_reference(harness, benchmark, preparation)
            if benchmark in harness.INPUTS:
                reference_paths.append(harness.INPUTS[benchmark][1])
            pins = preparation.merge(pins, {str(path): preparation.sha(path) for path in reference_paths})
            ids = [row['id'] for row in profiles]
            manifest = dict(status='prepared-awaiting-root-review', benchmark=benchmark, created_at=guard.now(),
                samples=3, size=harness.WORK[benchmark][1], canonical_full_workload=True,
                profiles=profiles, orders=[dict(round=i+1, profiles=ids[i:]+ids[:i]) for i in range(3)],
                pins=pins, shared_timing_lock=str(full_study.LOCK), full_output_reference=reference,
                maximum_rss_mib=10240, timeout_seconds=14400, maximum_compute_workers=8,
                required_power_profile=harness.REQUIRED, chart_admission=False, no_automatic_retry=True,
                source_revision=args.qualified_revision, full_processes=9,
                timing_boundary='Fresh process startup, all prescribed work, complete canonical stdin/stdout and shutdown; compilation, qualification and hashing excluded.',
                ownership='Supervisor, sampler and native process inherit the same common mutex descriptor. The frozen twenty-gate sampler records PGID ownership and cleans all owned children before release.',
                failure_policy='Verified genuine output/timeout/RSS source failures are preserved unscored while the other form/control finish. Integrity, environment and ownership errors stop for inspection; no automatic retry.',
                publication_status='Awaiting all full measurements and independent canonical LOC overlay')
            guard.verify(pins)
            path = output / benchmark / 'manifest.json'
            preparation.new_json(path, manifest)
            index['benchmarks'][benchmark] = dict(manifest=str(path), sha256=preparation.sha(path))
        preparation.new_json(output / 'index.json', index)
        print(json.dumps(dict(index=str(output / 'index.json'), sha256=preparation.sha(output / 'index.json'),
                              benchmarks=index['benchmarks'], full_processes=90), indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualified-revision', required=True)
    parser.add_argument('--study-revision', required=True)
    seal(parser.parse_args())


if __name__ == '__main__':
    main()
