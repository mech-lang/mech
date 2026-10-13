"""Seal twenty qualified exact Gleam binaries plus ten fresh unchanged Mech controls."""
import argparse
import copy
import fcntl
import hashlib
import json
from pathlib import Path

import full_study
import qualify_work

preparation = qualify_work.preparation
# Bind the reviewed generic service inside seal() after acquiring the mutex.
service = None
HERE = full_study.HERE


def validate_work(report_path, guard):
    report = json.loads(report_path.read_text())
    assert report['status'] == 'source-work-allocation-passed-awaiting-full-review-and-LOC'
    assert report['active'] is None and report['full_performance_runs'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    assert report['pins_unchanged_at_end'] is True and report['full_manifests_sealed'] == 0
    guard.verify(report['pins'])
    retained = report['preserved_failed_v3_work_qualification']
    assert retained['status'] == 'preserved-original-failed-v3-symbolic-local-call-parser'
    assert retained['failed_record_promoted_to_passed'] is retained['old_fixture_is_current_parser_qualification'] is False
    assert retained['original_observer_builds'] == retained['original_native_work_attempts'] == 0
    assert retained['failure_bytes_newly_pinned_under_current_mutex'] is True and retained['original_failed_report_had_terminal_pin_map'] is False
    old = Path(retained['report'])
    assert old == HERE/'work-qualification/work-v3/report.json'
    assert preparation.sha(old) == retained['report_sha256'] == report['pins'][str(old)]
    failure = json.loads(old.read_text())
    assert failure['status'] == 'stopped-unqualified' and failure['attempts'] == failure['proofs'] == []
    assert failure['error'] == 'Original scored creator must actually construct both children'
    assert failure['actual_mock_fixtures'] == retained['original_actual_fixture']
    fixture = report['actual_mock_fixtures']
    assert fixture['methods'] == 17 and fixture['zero_skips'] is True
    assert preparation.sha(fixture['receipt']) == fixture['sha256']
    qualify_work.fixture_runner.validate_receipt(json.loads(Path(fixture['receipt']).read_text()), 'work')
    prerequisite = Path(report['prerequisite'])
    assert preparation.sha(prerequisite) == report['prerequisite_sha256']
    qualified = qualify_work.qualified_report(prerequisite, guard)
    assert report['profiles'] == qualified['profiles']
    rows = {row['id']: row for row in report['profiles']}
    assert len(rows) == 20 and len(report['proofs']) == len(report['attempts']) == 48
    expected = set()
    for row in rows.values():
        if row['benchmark'] == 'binarytrees':
            expected.update(('tree', row['id'], 'allocated-n' + str(n)) for n in (0,4,7,9))
        elif row['benchmark'] == 'pidigits':
            expected.update(('pi', row['id'], 'real-divisions-n' + str(n)) for n in (0,1,9,10,11,27,97))
        elif row['benchmark'] == 'knucleotide':
            expected.update(('maps', row['id'], 'complete-maps-' + name)
                            for name, data in preparation.inputs('knucleotide', qualify_work.references))
    expected.update(('primitive', 'gleam-' + benchmark + '-compact', 'generic-abi')
                    for benchmark in ('pidigits','regexredux'))
    assert {(p['kind'],p['profile'],p['case']) for p in report['proofs']} == expected
    for proof in report['proofs']:
        row = rows[proof['profile']]
        assert proof['status'] == 'passed'
        assert proof['scored_source_sha256'] == row['source_sha256']
        assert proof['scored_binary_sha256'] == row['binary_sha256']
        assert proof['compiled_artifacts'] == row['compiled_artifacts']
        attempt = next(a for a in report['attempts'] if a['attempt'] == proof['attempt'])
        qualify_work.complete_attempt(attempt)
        assert attempt['source_sha256'] == proof['trace_source_sha256']
        assert attempt['id'] == proof['profile'] + '-work-trace' and attempt['case'] == proof['case']
        if proof['kind'] != 'primitive':
            assert proof['reconciliation']['status'] == 'passed'
        if proof['kind'] == 'tree':
            assert proof['allocation']['status'] == 'passed'
            assert all(proof['allocation'][key] is True for key in
                       ('fresh_reference_identity','both_actual_child_calls','dynamic_leaf_and_interior_tuples','retained_scored_long_lived_root'))
            assert all(proof['reconciliation'][key] is True for key in
                       ('actual_distinct_nodes_and_leaf_objects','actual_stretch_and_all_iteration_roots',
                        'unchanged_long_lived_graph_after_churn','joined_actual_worker_intervals',
                        'actual_interval_labels_correlated','concurrent_churn_disjoint_from_retained_graph'))
        elif proof['kind'] == 'pi':
            assert proof['reconciliation']['actual_both_division_operands_and_quotients'] is True
            assert proof['reconciliation']['independent_recurrence'] is True
        elif proof['kind'] == 'maps':
            assert all(proof['reconciliation'][key] is True for key in
                       ('actual_seven_complete_maps','complete_map_counts_and_window_totals_match','heap_owned_tables_after_actual_joins'))
            assert len(proof['reconciliation']['maps']) == 7
            assert proof['reviewed_source_loop']['status'] == 'passed'
            assert proof['reviewed_source_loop']['source_sha256'] == row['source_sha256']
            assert proof['reviewed_source_loop']['source'] == row['source']
            if proof['case'] == 'complete-maps-arbitrary-widths':
                assert proof['reconciliation']['source_map_growth'] is True
    assert len(report['source_reviews']) == 20 and {p['profile'] for p in report['source_reviews']} == set(rows)
    for review in report['source_reviews']:
        row = rows[review['profile']]
        assert review['status'] == 'passed' and review['scored_source_sha256'] == row['source_sha256']
        assert review['scored_binary_sha256'] == row['binary_sha256']
        assert review['compiled_source_bundle'] == row['source_bundle']
        assert review['worker_cap'] == row['maximum_compute_workers'] <= 8
        assert review['complete_bounded_cases'] == len([a for a in qualified['attempts'] if a['id'] == row['id']])+len(row.get('prior_bounded_records',[]))
    return report, service.merge(report['pins'], qualified['pins'],
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
    global service
    print('WAIT common mutex: Gleam exact full-study manifests', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        service = preparation.load_service()
        guard = service.load_guard()
        harness = guard.load_harness()
        work_path = HERE / 'work-qualification' / args.work_revision / 'report.json'
        work, common = validate_work(work_path, guard)
        audit_path = full_study.SUITE / 'source-audit-r.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        evidence = [Path(__file__).resolve(), Path(full_study.__file__).resolve(),
            Path(__file__).with_name('launch_all.py'), Path(__file__).with_name('test_full_study.py'),
            Path(qualify_work.__file__).resolve(), Path(qualify_work.work_traces.__file__).resolve(),
            Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'), full_study.GUARD,
            full_study.SUITE / 'run.py', full_study.SUITE / 'prepare.py', audit_path]
        common = service.merge(common, audit['pins'], {str(path): preparation.sha(path) for path in evidence})
        output = HERE / 'studies' / args.study_revision
        assert not output.exists(), 'No replacement or implicit resume of a sealed study'
        output.mkdir(parents=True)
        index = dict(status='sealed-awaiting-root-review', work_qualification=str(work_path),
            work_qualification_sha256=preparation.sha(work_path), samples=3, benchmarks={},
            full_processes=90, chart_admission=False, no_automatic_retry=True)
        for benchmark in full_study.BENCHMARKS:
            profiles = [copy.deepcopy(row) for row in work['profiles'] if row['benchmark'] == benchmark]
            assert len(profiles) == 2 and {row['variant'] for row in profiles} == {'compact', 'optimized'}
            pins = dict(common)
            for row in profiles:
                qualified_environment=dict(row['runtime_environment'])
                dump=output/benchmark/'runtime-output'/row['id']/'erl_crash.dump'
                dump.parent.mkdir(parents=True)
                row['qualified_runtime_environment']=qualified_environment
                row['runtime_environment']=dict(qualified_environment,ERL_CRASH_DUMP=str(dump))
                row['runtime_environment_overrides']={'ERL_CRASH_DUMP':{'from':qualified_environment.get('ERL_CRASH_DUMP'),'to':str(dump)}}
                row['execution_identity_status']='Same qualified source/binary/artifacts/classpath; explicit fresh crash-output environment, newly derived full execution fingerprint'
                row.update(binary=row['command'][0], maximum_workers=row['maximum_compute_workers'], loc=None,
                    producer_loc=None, loc_status='pending qualified canonical LOC overlay', eligible_for_chart=False,
                    status='qualified-source-awaiting-three-full-samples', qualification_status='source-output-work-allocation-passed',
                    chart_admission=False, role='Qualified complete Gleam ' + row['variant'] + ' application',
                    required_work_review=dict(status='passed', source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],
                        compiled_artifacts=row['compiled_artifacts'],runtime_classpath=row['runtime_classpath'],
                        compiled_input_inventory=row['compiled_input_inventory'],compiled_input_inventory_sha256=row['compiled_input_inventory_sha256'],
                        method='Exact source review and all bounded outputs; actual seven immutable BEAM maps/all windows/task ownership, both real Pi divisions and original scored BEAM dynamic tuple/reference allocation/retention proofs',
                        review=str(Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md')), qualification=str(work_path)))
            control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == benchmark))
            if benchmark == 'mandelbrot':
                path = full_study.CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
                study = json.loads(path.read_text())
                control = copy.deepcopy(next(row for row in study['profiles'] if row['id'] == 'row16-aot-control'))
                pins = service.merge(pins, study['pins'], {str(path): preparation.sha(path)})
            assert preparation.sha(control['source']) == control['source_sha256']
            assert preparation.sha(control['binary']) == control['binary_sha256']
            pins = service.merge(pins, control.get('support', {}),
                {str(Path(control['source'])): control['source_sha256'], str(Path(control['binary'])): control['binary_sha256']})
            control.update(id='mech-aot-control', key=benchmark + ':gleam-full-study:mech-control',
                producer_loc=control.get('loc'), loc=None, loc_status='pending qualified canonical LOC overlay',
                maximum_workers=8, role='Fresh unchanged current Mech AOT source control')
            profiles.append(control)
            reference, reference_paths = full_reference(harness, benchmark)
            if benchmark in harness.INPUTS:
                reference_paths.append(harness.INPUTS[benchmark][1])
            pins = service.merge(pins, {str(path): preparation.sha(path) for path in reference_paths})
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
    assert args.work_revision == 'work-v4'
    assert all(value and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in value)
               for value in (args.work_revision, args.study_revision))
    seal(args)


if __name__ == '__main__':
    main()
