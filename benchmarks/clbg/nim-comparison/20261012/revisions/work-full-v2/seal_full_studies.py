"""Seal twenty qualified exact Nim binaries plus ten fresh unchanged Mech controls."""
import argparse
import copy
import fcntl
import hashlib
import json
import os
import sys
from pathlib import Path

import full_study
import fixture_runner
import qualify_work

preparation = qualify_work.preparation
HERE = full_study.HERE


def validate_work(report_path, guard):
    report = json.loads(report_path.read_text())
    assert report['status'] == 'source-work-allocation-passed-awaiting-full-review-and-LOC'
    assert report['active'] is None and report['full_performance_runs'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    assert report['pins_unchanged_at_end'] is True and report['full_manifests_sealed'] == 0
    guard.verify(report['pins'])
    fixture=report['actual_mock_fixtures']
    assert fixture['methods']==12 and fixture['zero_skips'] is True
    assert preparation.sha(fixture['receipt'])==fixture['sha256']
    fixture_runner.validate_receipt(json.loads(Path(fixture['receipt']).read_text()),'work')
    prerequisite = Path(report['prerequisite'])
    assert preparation.sha(prerequisite) == report['prerequisite_sha256']
    qualified, reuse = qualify_work.qualified_report(prerequisite, guard)
    assert report['profiles'] == qualified['profiles']
    rows = {row['id']: row for row in report['profiles']}
    assert len(rows) == 20
    expected = {(row['id'], 'allocated-n' + str(n)) for row in rows.values()
                if row['benchmark'] == 'binarytrees' for n in (0, 4, 7, 9)}
    assert len(report['tree_proofs']) == len(report['attempts']) == len(expected) == 8
    assert {(proof['profile'], proof['case']) for proof in report['tree_proofs']} == expected
    for proof in report['tree_proofs']:
        row = rows[proof['profile']]
        assert proof['status'] == proof['allocation']['status'] == proof['reconciliation']['status'] == 'passed'
        assert proof['scored_source_sha256'] == row['source_sha256']
        assert proof['scored_binary_sha256'] == row['binary_sha256']
        assert all(proof['allocation'][field] is True for field in
            ('real_allocation_call', 'two_actual_child_calls', 'final_arm_leaf_branch','unconditional_allocation_before_control'))
        assert all(proof['reconciliation'][field] is True for field in
            ('distinct_nodes_and_all_leaves_per_actual_root', 'no_closed_form_substitute',
             'actual_stretch_and_all_iteration_roots', 'unchanged_long_lived_root_after_churn', 'actual_joined_worker_intervals',
             'actual_interval_labels_correlated','concurrent_churn_disjoint_from_retained_graph'))
        attempt = next(attempt for attempt in report['attempts'] if attempt['attempt'] == proof['attempt'])
        assert attempt['id'] == proof['profile'] + '-work-trace' and attempt['case'] == proof['case']
        assert attempt['source_sha256'] == proof['trace_source_sha256']
        assert attempt['qualified'] is True and attempt['exit_status'] == attempt['sampler_wait_status'] == 0
        assert set(attempt['qualification_checks']) == qualify_work.proof_reuse_v5.old.EXPECTED_GATES
        assert all(value is True for value in attempt['qualification_checks'].values())
    assert len(report['source_reviews']) == len(rows)
    assert {review['profile'] for review in report['source_reviews']} == set(rows)
    for review in report['source_reviews']:
        row = rows[review['profile']]
        assert review['status'] == 'passed' and review['scored_source_sha256'] == row['source_sha256']
        assert review['worker_cap'] == row['maximum_compute_workers'] <= 8
        assert review['complete_bounded_cases'] == len(row['prior_bounded_records'])
    return report, preparation.merge(report['pins'], qualified['pins'], reuse['pins'],
        {str(report_path): preparation.sha(report_path)})


def full_reference(harness, benchmark):
    if benchmark == 'nbody':
        return None, []
    if benchmark in ('pidigits', 'regexredux'):
        path = full_study.CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=path.stat().st_size, sha256=preparation.sha(path)), [path]
    if benchmark in ('spectralnorm', 'fannkuchredux'):
        output = {'spectralnorm': b'1.274224153\n',
                  'fannkuchredux': b'3968050\nPfannkuchen(12) = 65\n'}[benchmark]
        return dict(bytes=len(output), sha256=hashlib.sha256(output).hexdigest()), []
    size, digest = harness.compact.HASHES[benchmark][1]
    return dict(bytes=size, sha256=digest), []


def seal(args):
    print('WAIT common mutex: Nim exact full-study manifests', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        guard = preparation.load_guard()
        harness = guard.load_harness()
        work_path = HERE / 'work-qualification' / args.work_revision / 'report.json'
        work, common = validate_work(work_path, guard)
        fixture_receipt=Path(work['actual_mock_fixtures']['receipt'])
        fixture_runner.validate_receipt(json.loads(fixture_receipt.read_text()),'work')
        audit_path = full_study.SUITE / 'source-audit-r.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        evidence = [Path(__file__).resolve(), Path(full_study.__file__).resolve(),
            Path(__file__).with_name('launch_all.py'), Path(__file__).with_name('test_full_study.py'),
            Path(__file__).with_name('fixture_runner.py'), Path(__file__).with_name('REVISION.md'),
            Path(qualify_work.__file__).resolve(), Path(qualify_work.tree_work.__file__).resolve(),
            Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'), full_study.GUARD,
            full_study.SUITE / 'run.py', full_study.SUITE / 'prepare.py', audit_path]
        common = preparation.merge(common, audit['pins'], {str(path): preparation.sha(path) for path in evidence})
        output = HERE / 'studies' / args.study_revision
        assert not output.exists(), 'No replacement or implicit resume of a sealed study'
        output.mkdir(parents=True)
        index = dict(status='sealed-awaiting-root-review', work_qualification=str(work_path),
            work_qualification_sha256=preparation.sha(work_path), samples=3, benchmarks={},
            full_processes=90, chart_admission=False, no_automatic_retry=True,
            controller_revision='work-full-v2', actual_controller_fixture_receipt=str(fixture_receipt),
            actual_controller_fixture_receipt_sha256=preparation.sha(fixture_receipt),
            exact_controller_fixture_methods=12, zero_fixture_skips=True)
        for benchmark in full_study.BENCHMARKS:
            profiles = [copy.deepcopy(row) for row in work['profiles'] if row['benchmark'] == benchmark]
            assert len(profiles) == 2 and {row['variant'] for row in profiles} == {'compact', 'optimized'}
            pins = dict(common)
            for row in profiles:
                row.update(binary=row['command'][0], maximum_workers=row['maximum_compute_workers'], loc=None,
                    producer_loc=None, loc_status='pending qualified canonical LOC overlay', eligible_for_chart=False,
                    status='qualified-source-awaiting-three-full-samples', qualification_status='source-output-work-allocation-passed',
                    chart_admission=False, role='Qualified complete Nim ' + row['variant'] + ' application',
                    required_work_review=dict(status='passed', source_sha256=row['source_sha256'],
                        method='Exact source review and all bounded outputs; actual seven maps/ORC/ASAN, both pi operands and scored tree allocation/retention proofs',
                        review=str(Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md')), qualification=str(work_path)))
            control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == benchmark))
            if benchmark == 'mandelbrot':
                path = full_study.CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
                study = json.loads(path.read_text())
                control = copy.deepcopy(next(row for row in study['profiles'] if row['id'] == 'row16-aot-control'))
                pins = preparation.merge(pins, study['pins'], {str(path): preparation.sha(path)})
            assert preparation.sha(control['source']) == control['source_sha256']
            assert preparation.sha(control['binary']) == control['binary_sha256']
            pins = preparation.merge(pins, control.get('support', {}),
                {str(Path(control['source'])): control['source_sha256'], str(Path(control['binary'])): control['binary_sha256']})
            control.update(id='mech-aot-control', key=benchmark + ':nim-full-study:mech-control',
                producer_loc=control.get('loc'), loc=None, loc_status='pending qualified canonical LOC overlay',
                maximum_workers=8, role='Fresh unchanged current Mech AOT source control')
            profiles.append(control)
            reference, reference_paths = full_reference(harness, benchmark)
            if benchmark in harness.INPUTS:
                reference_paths.append(harness.INPUTS[benchmark][1])
            pins = preparation.merge(pins, {str(path): preparation.sha(path) for path in reference_paths})
            ids = [row['id'] for row in profiles]
            manifest = dict(status='prepared-awaiting-root-review', benchmark=benchmark, created_at=guard.now(),
                samples=3, size=harness.WORK[benchmark][1], canonical_full_workload=True, profiles=profiles,
                orders=[dict(round=i + 1, profiles=ids[i:] + ids[:i]) for i in range(3)], pins=pins,
                shared_timing_lock=str(full_study.LOCK), full_output_reference=reference,
                maximum_rss_mib=10240, timeout_seconds=14400, maximum_compute_workers=8,
                required_power_profile=harness.REQUIRED, chart_admission=False, no_automatic_retry=True,
                source_revision=work['qualification_revision'], work_revision=args.work_revision, full_processes=9,
                timing_boundary='Fresh process startup, all prescribed work, complete canonical stdin/stdout and shutdown; compilation, qualification and hashing excluded.',
                ownership='Supervisor, sampler and native process inherit the common mutex descriptor; frozen twenty-gate PGID ownership and cleanup before release.',
                publication_status='Pending all full measurements and independent canonical LOC overlay')
            guard.verify(pins)
            path = output / benchmark / 'manifest.json'
            preparation.new_json(path, manifest)
            index['benchmarks'][benchmark] = dict(manifest=str(path), sha256=preparation.sha(path))
        preparation.new_json(output / 'index.json', index)
        print(json.dumps(dict(index=str(output / 'index.json'), sha256=preparation.sha(output / 'index.json'),
            benchmarks=index['benchmarks'], full_processes=90), indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--work-revision', required=True)
    parser.add_argument('--study-revision', required=True)
    args = parser.parse_args()
    assert all(value and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in value)
               for value in (args.work_revision, args.study_revision))
    seal(args)


if __name__ == '__main__':
    main()
