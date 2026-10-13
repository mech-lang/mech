"""Actual two-Pi/14-fixture reuse; call only after common-lock packet pinning."""
import copy
import json
from pathlib import Path

OLD_MANIFEST_SHA = '19df80734fd7d294d9c6e36ad356bb37cf1acd0d93dd3a755f2eec4bbb33d26d'
QUALIFIED_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
PI_IDS = ('pony-pidigits-compact', 'pony-pidigits-optimized')
FIXTURE_CLOSURE = ('run_work_fixtures.py', 'test_full_study.py', 'full_study.py',
    'test_work_v2.py', 'check_traces.py', 'check_traces_base.py',
    'test_owned_work_service.py', 'owned_service_work_v2.py')
FIXTURE_METHODS = {
    'full-gates': ('test_full_study.IsolatedFailures.', (
        'test_real_source_failure_with_literal_safe_gates_is_isolated',
        'test_every_unsafe_gate_stops_and_truthy_values_do_not_waive_it',
        'test_missing_gates_and_non_source_failure_stop')),
    'tree-identity': ('test_work_v2.TreeProofV2.', (
        'test_complete_graph_and_lifecycle', 'test_early_retained_node_finalizer_rejected',
        'test_shared_leaf_rejected', 'test_missing_worker_join_rejected',
        'test_skipped_actual_check_rejected', 'test_live_identity_reuse_rejected',
        'test_released_stretch_identity_reuse_is_allowed')),
    'owned-service': ('test_owned_work_service.OwnedService.', (
        'test_success_requires_bounded_reap_and_empty_group',
        'test_failed_group_kill_and_wait_preserve_bounded_receipt',
        'test_missing_live_identity_still_cleans_and_journals',
        'test_resource_observer_failure_after_journal_cleans_known_child')),
}


def receipt(path, command, status):
    value = json.loads(path.read_text())
    assert value['command'] == command
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == status
    assert value['inherited_common_mutex'] is value['bounded_local_service'] is True
    assert value['leader_reaped'] is value['cleanup_complete'] is True
    assert value['remaining_group_members'] == value['cleanup_errors'] == []
    assert not value.get('cleanup_error')
    if status == 0:
        assert not value.get('error')
    else:
        assert value['error'] == "('Owned command failed', 'trace-build', 255)"
    return value


def reconstruct(preparation, guard, harness, generator, checker, references, packet_pins):
    cohort = preparation.HERE
    old = cohort / 'work-qualification/private-v8-work-v2'
    old_packet = cohort / 'revisions/work-full-v2'
    packet = Path(__file__).resolve().parent
    manifest_path, report_path = old / 'manifest.json', old / 'report.json'
    assert preparation.sha(manifest_path) == OLD_MANIFEST_SHA
    manifest = json.loads(manifest_path.read_text())
    assert manifest['status'] == 'prepared-awaiting-root-review'
    assert manifest['qualification'] == str(cohort / 'qualification/private-v8/report.json')
    assert manifest['qualification_sha256'] == QUALIFIED_SHA
    assert preparation.sha(manifest['qualification']) == QUALIFIED_SHA
    guard.verify(manifest['pins'])
    prior = json.loads(report_path.read_text())
    assert prior['status'] == 'stopped' and prior['error_type'] == 'AssertionError'
    assert prior['error'] == "('Owned command failed', 'trace-build', 255)"
    assert prior['manifest'] == str(manifest_path) and prior['manifest_sha256'] == OLD_MANIFEST_SHA
    assert type(prior['full_performance_runs']) is int and prior['full_performance_runs'] == 0
    assert tuple(item['id'] for item in prior['attempts']) == PI_IDS
    assert 'scored_tree_raw_allocation_evidence' not in prior
    assert not (old / 'scored-tree-allocation').exists(), 'No prior ARM/IR operation may be silently dropped'
    pins = preparation.merge(manifest['pins'], preparation.pin_tree(old))
    for name in FIXTURE_CLOSURE:
        assert preparation.sha(packet / name) == packet_pins[str(packet / name)] == manifest['pins'][str(old_packet / name)]
    fixtures = []
    for suite, (prefix, methods) in FIXTURE_METHODS.items():
        path = old / 'fixtures' / suite / (suite + '-fixture-receipt.json')
        value = json.loads(path.read_text()); expected = sorted(prefix + name for name in methods)
        assert value['status'] == 'passed' and value['suite'] == suite
        assert type(value['tests_run']) is type(value['expected_count']) is int
        assert value['tests_run'] == value['expected_count'] == len(expected)
        assert sorted(value['expected_ids']) == sorted(value['discovered_ids']) == sorted(value['started_ids']) == expected
        assert len(value['started_ids']) == len(set(value['started_ids'])) == len(expected)
        assert value['actual_common_mutex_held'] is value['inherited_common_mutex'] is True
        for key in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes', 'native_probes'):
            assert type(value[key]) is int and value[key] == 0
        assert value['chart_admission'] is False
        assert preparation.sha(value['raw_log']) == value['raw_log_sha256'] == pins[value['raw_log']]
        fixtures.append(dict(path=str(path), sha256=pins[str(path)], suite=suite, actual_tests=len(expected), raw_log=value['raw_log']))
    rows = {row['id']: row for row in manifest['rows']}
    assert len(rows) == len(manifest['rows']) == 6
    compiler = Path(manifest['toolchain']['compiler'])
    sdk = Path(manifest['toolchain']['sdk'])
    regex = Path(manifest['toolchain']['regex_root'])
    gmp = Path(manifest['toolchain']['gmp_root'])
    failed_id = 'pony-knucleotide-compact-precedence-v4-trace'
    failed_source = Path(rows['pony-knucleotide-compact-precedence-v4']['trace_source'])
    failed_command = preparation.build_command(compiler, sdk, regex, gmp, failed_source, old / 'bin' / failed_id, failed_id)
    failed_build = old / 'builds' / failed_id / 'trace-build.json'
    receipt(failed_build, failed_command, 255)
    failure_stderr = failed_build.with_suffix('.stderr')
    failure_text = failure_stderr.read_text()
    assert 'counts.pairs()' in failure_text and 'receiver type: Map[String val, U64 val] iso' in failure_text
    assert 'iso is not a subcap of box^' in failure_text
    assert not (old / 'executions' / failed_id).exists()
    assert not (old / 'bin' / failed_id / failed_id).exists()
    retained = []
    for attempt in prior['attempts']:
        row = rows[attempt['id']]; identifier = row['id'] + '-trace'
        assert attempt['benchmark'] == row['benchmark'] == 'pidigits'
        assert attempt['source'] == row['source'] and attempt['source_sha256'] == row['source_sha256']
        source, trace = Path(row['source']), Path(row['trace_source'])
        generated, _ = generator.instrument(source.read_text(), row['benchmark'], row['variant'])
        assert generated == trace.read_text()
        assert preparation.sha(trace) == attempt['trace_source_sha256'] == pins[str(trace)]
        assert preparation.sha(source) == attempt['source_sha256'] == pins[str(source)]
        binary = old / 'bin' / identifier / identifier
        assert preparation.sha(binary) == attempt['binary_sha256'] == pins[str(binary)]
        command = preparation.build_command(compiler, sdk, regex, gmp, trace, binary.parent, identifier)
        receipt(old / 'builds' / identifier / 'trace-build.json', command, 0)
        image = old / 'images' / identifier
        receipt(image / 'architecture.json', ['/usr/bin/lipo', '-archs', str(binary)], 0)
        receipt(image / 'linked-images.json', ['/usr/bin/otool', '-L', str(binary)], 0)
        dependency_review = json.loads((image / 'dependency-review.json').read_text())
        assert dependency_review['binary'] == str(binary) and 'arm64' in dependency_review['architectures']
        for item in dependency_review['dependencies']:
            if item.get('scope') == 'OS-image-not-hashed':
                assert item['install_name'].startswith(('/usr/lib/', '/System/Library/'))
            else:
                assert preparation.sha(item['path']) == item['sha256'] == pins[item['path']]
        native_command = [str(binary), '97', '--ponymaxthreads', '8']
        execution = old / 'executions' / identifier
        native_receipt = receipt(execution / 'trace-native.json', native_command, 0)
        actual = attempt['observations']
        assert actual == json.loads((execution / 'observations.json').read_text())
        assert actual['power_at_start'] == actual['power_at_end'] == harness.REQUIRED
        assert not any(actual[key] for key in ('competing_compilers', 'power_changes', 'monitor_errors', 'resource_monitor_errors', 'resource_violation'))
        assert actual['native']['command'] == native_command
        assert type(actual['native']['pid']) is type(actual['native']['pgid']) is int
        assert actual['native']['pid'] == actual['native']['pgid'] == native_receipt['pid']
        assert actual['native']['inherited_common_mutex'] is actual['native']['observer_registered_after_owned_active_journal'] is True
        assert attempt['input_n'] == 97 and attempt['stdin'] is None
        assert attempt['qualified'] is attempt['complete_output_match'] is True
        trace_path = execution / 'trace-native.stderr'
        assert attempt['trace'] == str(trace_path) and attempt['trace_sha256'] == preparation.sha(trace_path) == pins[str(trace_path)]
        assert (execution / 'trace-native.stdout').read_bytes() == references.pidigits(97)
        proof = checker.check('pidigits', trace_path.read_bytes(), 97, None, references, row['variant'])
        mutations = attempt['proof']['actual_record_mutation_rejections']
        assert mutations == ['missing-work-record', 'too-many-scheduler-identities']
        assert dict(proof, actual_record_mutation_rejections=mutations) == attempt['proof']
        raw_lines = trace_path.read_bytes().splitlines(keepends=True)
        first = next(index for index, line in enumerate(raw_lines) if not line.startswith(b'THREAD\t'))
        invalid = (b''.join(raw_lines[:first] + raw_lines[first + 1:]),
            trace_path.read_bytes() + b''.join(b'THREAD\t' + str(index).encode() + b'\n' for index in range(1001, 1010)))
        for changed in invalid:
            try:
                checker.check('pidigits', changed, 97, None, references, row['variant'])
            except (AssertionError, KeyError, IndexError):
                pass
            else:
                raise AssertionError('Retained Pi mutation must remain rejected')
        retained.append(copy.deepcopy(attempt))
    guard.verify(pins)
    return dict(status='verified-two-actual-Pi-work-proofs-and-fourteen-fixtures',
        prior_report=dict(path=str(report_path), sha256=pins[str(report_path)]),
        prior_manifest=dict(path=str(manifest_path), sha256=OLD_MANIFEST_SHA),
        failed_compact_K_build=dict(path=str(failed_build), sha256=pins[str(failed_build)],
            stderr=str(failure_stderr), stderr_sha256=pins[str(failure_stderr)]),
        attempts=retained, reused_actual_work_traces=2, new_actual_work_traces_required=4,
        fixtures=fixtures, reused_fixture_tests=14, production_cases_reexecuted=0,
        old_attempts_relabelled_as_new=False, prior_tree_traces=0, prior_scored_ARM_IR_operations=0,
        pins=pins, chart_admission=False)
