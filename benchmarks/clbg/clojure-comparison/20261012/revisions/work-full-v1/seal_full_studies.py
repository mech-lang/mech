"""Unexecuted Clojure full-manifest candidate; no compiler/native launch.

Requires actual separately reviewed work evidence, exact first-packet bounded
proof, and passed mocked ledger fixtures. Every hash/write holds common mutex.
"""
import argparse
import copy
import fcntl
import json
import os
from pathlib import Path
import re

import full_study
import qualify_work

HERE = full_study.HERE
CLBG = full_study.CLBG
GATES = full_study.GATES
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
PREPARATION = qualify_work.PREPARATION
PROOF_FIELDS = {
    'binarytrees': {'actual_fresh_node_storage', 'actual_leaf_allocation',
        'retained_identity_after_churn', 'all_iteration_graphs_verified', 'worker_cap_verified'},
    'knucleotide': {'all_seven_actual_maps', 'all_windows_and_complete_inventories',
        'actual_growth_observed', 'private_workers_and_same_map_handoff'},
    'pidigits': {'both_actual_extraction_divisions', 'accepted_and_rejected_recurrence',
        'zero_digits_no_extraction'},
}
FIXTURE_METHODS = [
    'test_nine_actual_ledger_entries_three_each_and_matched_control',
    'test_unqualified_second_attempt_persists_and_stops_without_retry',
    'test_verified_source_failure_is_unscored_and_other_form_control_finish',
    'test_isolation_requires_every_literal_safety_gate_and_exact_typed_inventory',
    'test_supervisor_exception_preserves_raw_and_separate_unqualified_wrapper',
    'test_interrupt_becomes_durable_stopped_evidence_without_actual_signals',
    'test_exact_twenty_gate_names_and_literal_true_values_are_required',
]
PROOF_FIXTURE_METHODS = [
    'test_tree_requires_actual_fresh_leaves_and_distinct_graph_storage',
    'test_retained_graph_identity_must_survive_actual_churn',
    'test_all_seven_growing_map_inventories_and_handoff_are_required',
    'test_every_overlapping_window_including_tail_is_required',
    'test_two_actual_pi_divisions_recurrence_and_zero_extraction_are_required',
    'test_native_mutex_description_and_owned_observer_cleanup_are_required',
]


def exact_true_checks(attempt):
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is False
    assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert attempt['validation']['reference_match'] is True and attempt['pins_unchanged_at_end'] is True
    assert attempt['full_performance_run'] is False


def merge(target, values):
    for path, digest in values.items():
        assert path not in target or target[path] == digest, ('Conflicting source/evidence pin', path)
        target[path] = digest


def evidence(reference, pins):
    path, digest = Path(reference['path']), reference['sha256']
    assert pins.get(str(path)) == digest and full_study.sha(path) == digest
    return path


def fixture(pins, reference=None):
    path = evidence(reference, pins) if reference else Path(__file__).with_name('full-study-fixtures.json')
    value = json.loads(path.read_text())
    assert value['status'] == 'passed' and value['tests_run'] == value['discovered_tests'] == 7
    expected = sorted('test_full_study.LedgerChecks.' + name for name in FIXTURE_METHODS)
    assert value['expected_test_ids'] == expected
    assert sorted(value['discovered_test_ids']) == sorted(value['started_test_ids']) == expected
    assert sorted(value['actual_test_ids']) == expected and len(set(value['actual_test_ids'])) == 7
    for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes', 'native_probes', 'actual_signals_sent'):
        assert type(value[name]) is int and value[name] == 0
    assert value['exclusive_mutex_held'] is True and value['shared_timing_lock'] == str(full_study.LOCK)
    merge(pins, value['pins'])
    merge(pins, {str(path): full_study.sha(path)})


def validate_reports(args, guard):
    assert full_study.sha(args.qualified_file) == args.qualified_sha256
    assert full_study.sha(args.work_file) == args.work_sha256
    qualified, preparation = qualify_work.sealed_preparation(args.qualified_file, args.qualified_sha256)
    work = json.loads(args.work_file.read_text())
    assert qualified['status'] == 'bounded-output-ownership-passed-awaiting-actual-work-and-full-review'
    assert qualified['pins_unchanged_at_end'] is True and qualified['active'] is None
    assert qualified['full_performance_runs'] == qualified['full_manifests_sealed'] == 0
    assert work['status'] == 'passed-exact-source-compiled-JVM-work-review'
    assert work['pins_unchanged_at_end'] is True and work['active'] is None
    assert work['full_performance_runs'] == 0 and work['unresolved'] == []
    assert work['review_identity'] and work['scored_sources_and_classes_unchanged'] is True
    assert work['qualified_preparation'] == dict(path=str(args.qualified_file), sha256=args.qualified_sha256)
    pins = dict(qualified['pins'])
    merge(pins, work['pins'])
    merge(pins, {str(args.qualified_file): args.qualified_sha256, str(args.work_file): args.work_sha256})
    guard.verify(pins)
    assert str(PREPARATION) in pins, 'Actual qualified continuation controller is required'
    runtime_path = args.qualified_file.with_name('runtime-receipt.json')
    assert str(runtime_path) in pins
    runtime = json.loads(runtime_path.read_text())
    assert runtime['status'] == 'qualified-native-arm64-jvm-identity'
    assert runtime['compiler_version'] == '1.12.6' and runtime['java_runtime_version'] == preparation.JDK_VERSION
    assert runtime['measured_architecture'] == 'arm64' and runtime['collector'] == 'G1'
    assert runtime['backend'] == 'JVM bytecode then HotSpot JIT' and runtime['flags'] == preparation.JVM_FLAGS
    assert len(runtime['vendor_jars']) == 3
    java = Path(runtime['java'])
    jars = [Path(row['path']) for row in runtime['vendor_jars']]
    assert all(pins[str(row['path'])] == row['sha256'] for row in runtime['vendor_jars'])
    assert full_study.sha(java) == pins[str(java)]
    rows = qualified['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {
        (benchmark, variant) for benchmark in full_study.BENCHMARKS for variant in ('compact', 'optimized')}
    assert set(work['profiles']) == {row['id'] for row in rows}
    proposal_path = Path(work['first_proposal']['path'])
    evidence(work['first_proposal'], pins)
    assert work['first_proposal']['sha256'] == qualified['proposal_sha256']
    proposal = json.loads(proposal_path.read_text())
    attempts = qualified['attempts']
    assert len(attempts) == len({attempt['attempt'] for attempt in attempts})
    for attempt in attempts:
        exact_true_checks(attempt)
    assert work['attempts'] and len(work['attempts']) == len({attempt['attempt'] for attempt in work['attempts']})
    for attempt in work['attempts']:
        exact_true_checks(attempt)
    consumed_work_attempts = set()
    for row in rows:
        identity = row['id']
        review = work['profiles'][identity]
        assert review['status'] == 'passed' and review['review_identity']
        assert review['source_sha256'] == row['source_sha256'] and review['binary_sha256'] == row['binary_sha256']
        assert review['compiled_input_inventory_sha256'] == row['compiled_input_inventory_sha256']
        assert review['complete_static_source_review'] is True and review['unresolved'] == []
        evidence(review['semantic_source_review'], pins)
        assert row['binary'] == str(java) and row['binary_sha256'] == pins[str(java)]
        assert row['maximum_compute_workers'] <= 8
        assert row['runtime_environment'] == preparation.controlled_environment(Path(runtime['java_home']))
        classpath = [Path(part) for part in row['runtime_classpath']]
        assert classpath[1:] == jars and classpath[0].name == identity
        assert row['command'] == preparation.runtime_command(java, classpath[0], jars, row['namespace'])
        inventory_path = Path(row['compiled_input_inventory'])
        assert pins[str(inventory_path)] == row['compiled_input_inventory_sha256']
        inventory = json.loads(inventory_path.read_text())
        assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == identity
        assert inventory['source_bundle'] == row['source_bundle'] and inventory['compiled_artifacts'] == row['compiled_artifacts']
        assert inventory['runtime_classpath'] == row['runtime_classpath']
        assert inventory['source_sha256'] == row['source_sha256'] and inventory['source'] == row['source']
        assert len(row['source_bundle']) == 2 and [entry['role'] for entry in row['source_bundle']] == ['application', 'application-helper']
        assert Path(row['source_bundle'][1]['path']).name == 'support.clj'
        package = Path(row['source']).parents[3]
        command = preparation.compile_command(java, package, classpath[0], jars, row['namespace'])
        assert inventory['build_command'] == command
        compile_path = Path(row['compile_receipt'])
        # Additive preparation retains original successful packages/classes and
        # their original owned build receipts. Never reconstruct a fresh receipt
        # pathname for those rows or relabel old compilation as a new operation.
        assert row['bounded_origin'] in {'retained-private-v1', 'fresh-private-v2'}
        origin = 'private-v1' if row['bounded_origin'] == 'retained-private-v1' else 'private-v2'
        assert compile_path == HERE / 'qualification' / origin / 'builds' / identity / 'compile.json'
        assert pins[str(compile_path)] == row['compile_receipt_sha256']
        assert full_study.sha(compile_path) == row['compile_receipt_sha256']
        assert str(compile_path) in pins
        compiled = json.loads(compile_path.read_text())
        assert compiled['command'] == command and compiled['inherited_common_mutex'] is True
        assert compiled['exit_status'] == compiled['final_exit_status'] == 0
        assert not any(compiled.get(key) for key in ('error', 'cleanup_error', 'cleanup_errors'))
        for entry in row['source_bundle']:
            assert pins[entry['path']] == entry['sha256'] and full_study.sha(entry['path']) == entry['sha256']
        assert all(pins[path] == digest and Path(path).suffix == '.class' for path, digest in row['compiled_artifacts'].items())
        observed = [attempt for attempt in attempts if attempt['id'] == identity]
        expected = proposal['case_inventory'][row['benchmark']]
        assert len(observed) == len({attempt['case'] for attempt in observed}) == len(expected)
        assert {attempt['case'] for attempt in observed} == set(expected)
        assert all(attempt['source_sha256'] == row['source_sha256'] and
            attempt['execution_fingerprint'] == guard.load_harness().execution_fingerprint(row) for attempt in observed)
        if row['benchmark'] in PROOF_FIELDS:
            flags = review['actual_work_flags']
            assert set(flags) == PROOF_FIELDS[row['benchmark']] and all(value is True for value in flags.values())
            assert review['bytecode_review']['status'] == 'passed'
            evidence(review['bytecode_review'], pins)
            evidence(review['observer_compiled_input_inventory'], pins)
            cases = expected if row['benchmark'] == 'knucleotide' else (
                ['allocated-n0', 'allocated-n7', 'allocated-n9'] if row['benchmark'] == 'binarytrees' else ['digits-97', 'zero-digits'])
            observations = review['observations']
            assert len(observations) == len({item['case'] for item in observations}) == len(cases)
            assert {item['case'] for item in observations} == set(cases)
            for observation in observations:
                assert observation['status'] == 'passed' and observation['scored_source_sha256'] == row['source_sha256']
                evidence(observation['edit_inventory'], pins)
                evidence(observation['proof'], pins)
                attempt = next(item for item in work['attempts'] if item['attempt'] == observation['attempt'])
                assert attempt['attempt'] not in consumed_work_attempts
                consumed_work_attempts.add(attempt['attempt'])
                exact_true_checks(attempt)
                assert attempt['id'] == identity + '-work-observer' and attempt['case'] == observation['case']
                assert attempt['source_sha256'] == observation['observer_source_sha256']
                assert attempt['source_work_proof']['status'] == 'passed'
    assert consumed_work_attempts == {attempt['attempt'] for attempt in work['attempts']}
    native_fd = work['native_mutex_fd_proof']
    assert native_fd['status'] == 'passed' and native_fd['actual_native_binary'] == str(java)
    assert native_fd['actual_native_binary_sha256'] == pins[str(java)]
    assert native_fd['same_common_open_file_description'] is True and native_fd['bounded_owned_observer_cleanup'] is True
    assert type(native_fd['actual_native_pid']) is int and native_fd['actual_native_pid'] > 0
    assert type(native_fd['actual_native_pgid']) is int and native_fd['actual_native_pgid'] > 0
    evidence(native_fd, pins)
    evidence(work['proof_parser_fixture_receipt'], pins)
    proof_fixture = json.loads(Path(work['proof_parser_fixture_receipt']['path']).read_text())
    assert proof_fixture['status'] == 'passed' and proof_fixture['exclusive_mutex_held'] is True
    assert proof_fixture['shared_timing_lock'] == str(full_study.LOCK)
    expected_proofs = sorted('test_work_proofs.WorkProofChecks.' + name for name in PROOF_FIXTURE_METHODS)
    assert proof_fixture['expected_test_ids'] == expected_proofs
    assert sorted(proof_fixture['discovered_test_ids']) == sorted(proof_fixture['started_test_ids']) == expected_proofs
    assert sorted(proof_fixture['actual_test_ids']) == expected_proofs
    assert proof_fixture['tests_run'] == proof_fixture['discovered_tests'] == len(proof_fixture['actual_test_ids']) == len(set(proof_fixture['actual_test_ids'])) == 6
    for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'):
        assert type(proof_fixture[name]) is int and proof_fixture[name] == 0
    assert proof_fixture['actual_native_subjects'] == proof_fixture['actual_signals_sent'] == 0
    fixture(pins, work['full_pipeline_fixture_receipt'])
    guard.verify(pins)
    return preparation, qualified, work, pins


def full_reference(harness, benchmark):
    if benchmark == 'nbody':
        return None, []
    if benchmark in ('pidigits', 'regexredux'):
        path = CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=path.stat().st_size, sha256=full_study.sha(path)), [path]
    if benchmark in ('spectralnorm', 'fannkuchredux'):
        import hashlib
        value = {'spectralnorm': b'1.274224153\n', 'fannkuchredux': b'3968050\nPfannkuchen(12) = 65\n'}[benchmark]
        return dict(bytes=len(value), sha256=hashlib.sha256(value).hexdigest()), []
    size, digest = harness.compact.HASHES[benchmark][1]
    return dict(bytes=size, sha256=digest), []


def seal(args):
    print('WAIT common mutex: Clojure exact work-qualified full manifests', flush=True)
    with full_study.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert full_study.sha(full_study.GUARD) == full_study.GUARD_SHA
        guard = full_study.module('clojure_frozen_full_sealer_guard', full_study.GUARD)
        preparation, qualified, work, pins = validate_reports(args, guard)
        harness = guard.load_harness()
        audit_path = SUITE / 'source-audit-r.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        merge(pins, audit['pins'])
        packet = [Path(__file__).resolve(), Path(full_study.__file__).resolve(),
            Path(__file__).with_name('test_full_study.py'), Path(__file__).with_name('REVIEW.md'),
            full_study.GUARD, audit_path, SUITE / 'run.py', SUITE / 'prepare.py']
        merge(pins, {str(path): full_study.sha(path) for path in packet})
        guard.verify(pins)
        output = HERE / 'studies' / args.study_revision
        assert not output.exists(), 'Exclusive full study; no replacement or implicit retry'
        manifests = {}
        for benchmark in full_study.BENCHMARKS:
            selected = [copy.deepcopy(row) for row in qualified['profiles'] if row['benchmark'] == benchmark]
            current = dict(pins)
            for row in selected:
                row.update(loc=None, producer_loc=None, loc_status='pending separate exact authored-main-and-helper LOC overlay',
                    maximum_workers=row['maximum_compute_workers'], eligible_for_chart=False, chart_admission=False,
                    required_work_review=dict(status='passed', source_sha256=row['source_sha256'],
                        review_identity=work['profiles'][row['id']]['review_identity'], evidence=str(args.work_file),
                        evidence_sha256=args.work_sha256), backend='JVM HotSpot JIT', platform='macos-arm64-jvm',
                    role='Qualified complete Clojure ' + row['variant'] + ' main and authored support')
            control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == benchmark))
            if benchmark == 'mandelbrot':
                path = CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
                previous = json.loads(path.read_text())
                control = copy.deepcopy(next(row for row in previous['profiles'] if row['id'] == 'row16-aot-control'))
                merge(current, previous['pins'])
                merge(current, {str(path): full_study.sha(path)})
            assert full_study.sha(control['source']) == control['source_sha256'] and full_study.sha(control['binary']) == control['binary_sha256']
            merge(current, control.get('support', {}))
            merge(current, {control['source']: control['source_sha256'], control['binary']: control['binary_sha256']})
            control.update(id='mech-aot-control', key=benchmark + ':clojure-full-study:mech-control',
                producer_loc=control.get('loc'), loc=None, loc_status='pending canonical count overlay', maximum_workers=8,
                control_for='clojure-full-study', role='Fresh unchanged current fastest Mech AOT source control')
            selected.append(control)
            reference, paths = full_reference(harness, benchmark)
            if benchmark in harness.INPUTS:
                paths.append(harness.INPUTS[benchmark][1])
            merge(current, {str(path): full_study.sha(path) for path in paths})
            ids = [row['id'] for row in selected]
            manifest = dict(status='prepared-awaiting-root-review', created_at=guard.now(), benchmark=benchmark,
                samples=3, size=harness.WORK[benchmark][1], canonical_full_workload=True, profiles=selected,
                orders=[dict(round=i+1, profiles=ids[i:]+ids[:i]) for i in range(3)], pins=current,
                shared_timing_lock=str(full_study.LOCK), full_output_reference=reference, full_processes=9,
                maximum_rss_mib=10240, timeout_seconds=14400, maximum_compute_workers=8,
                required_power_profile=harness.REQUIRED, chart_admission=False, no_automatic_retry=True,
                qualified_preparation=dict(path=str(args.qualified_file), sha256=args.qualified_sha256),
                actual_work_review=dict(path=str(args.work_file), sha256=args.work_sha256),
                timing_boundary='Fresh native ARM64 JVM startup, HotSpot JIT inside each sample, all prescribed work, complete canonical stdin/stdout, GC and shutdown; prior AOT bytecode compilation/qualification/hashing excluded.',
                ownership='The frozen exact twenty-gate supervisor/sampler/native group inherits the actual common mutex descriptor and completes owned cleanup before release.',
                failure_policy='Verified source output/timeout/RSS failures remain unscored with explicit skipped planned slots; other form/control continue. Integrity, environment and ownership failure stops. No automatic retry.',
                publication_status='Awaiting actual full measurements and separately qualified Clojure main/support LOC overlay')
            guard.verify(current)
            manifests[benchmark] = manifest
        output.mkdir(parents=True)
        index = dict(status='sealed-awaiting-root-review', qualified_preparation=str(args.qualified_file),
            qualified_preparation_sha256=args.qualified_sha256, actual_work_review=str(args.work_file),
            actual_work_review_sha256=args.work_sha256, samples=3, benchmarks={}, full_processes=90,
            chart_admission=False, no_automatic_retry=True)
        for benchmark, manifest in manifests.items():
            path = output / benchmark / 'manifest.json'
            preparation.new_json(path, manifest)
            index['benchmarks'][benchmark] = dict(manifest=str(path), sha256=full_study.sha(path))
        preparation.new_json(output / 'index.json', index)
        print(json.dumps(dict(index=str(output / 'index.json'), sha256=full_study.sha(output / 'index.json'),
            benchmarks=index['benchmarks'], full_processes=90), indent=2), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualified-file', type=Path, required=True)
    parser.add_argument('--qualified-sha256', required=True)
    parser.add_argument('--work-file', type=Path, required=True)
    parser.add_argument('--work-sha256', required=True)
    parser.add_argument('--study-revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.study_revision)
    for digest in (args.qualified_sha256, args.work_sha256):
        assert re.fullmatch(r'[0-9a-f]{64}', digest)
    args.qualified_file = args.qualified_file.resolve(strict=True)
    args.work_file = args.work_file.resolve(strict=True)
    assert args.qualified_file.is_relative_to(HERE / 'qualification')
    assert args.work_file.is_relative_to(HERE / 'work-qualification')
    seal(args)


if __name__ == '__main__':
    main()
