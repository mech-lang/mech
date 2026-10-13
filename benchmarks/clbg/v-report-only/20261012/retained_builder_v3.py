"""Reference unchanged passed V output evidence; execute nothing and copy no rows."""
import fcntl
import json
import math
import os
from pathlib import Path

import prepare_input_v2

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/production-input-v2'
RETAINED = ('regexredux', 'revcomp')
POWER = dict(source='AC Power', low_power_mode=0)


def validate_records(report, declared, benchmark, harness, digest, load):
    assert benchmark in RETAINED
    assert report['status'] == 'bounded-outputs-passed' and report['active'] is None
    assert report['benchmark'] == benchmark and report['revision'] == 'production-input-v2'
    assert report['full_performance_runs'] == 0 and report['never_chart_eligible'] is True
    assert report['pins_unchanged_at_end'] is True and not report.get('error')
    expected = {row['id']: row for row in declared if row['benchmark'] == benchmark}
    profiles = {row['id']: row for row in report['profiles']}
    assert len(expected) == len(profiles) == len(report['profiles']) == 2 and set(profiles) == set(expected)
    cases = {'canonical-small'} | {name for name, _ in prepare_input_v2.input_cases(benchmark)}
    assert len(report['attempts']) == 2 * len(cases)
    observed = set()
    for row in profiles.values():
        original = expected[row['id']]
        assert row['variant'] == original['variant'] and row['benchmark'] == benchmark
        assert row['original_source'] == str((HERE / original['source']).resolve())
        assert digest(row['source']) == digest(row['original_source']) == row['source_sha256']
        assert digest(row['binary']) == row['binary_sha256']
        assert row['source'] in report['pins'] and report['pins'][row['source']] == row['source_sha256']
        assert row['binary'] in report['pins'] and report['pins'][row['binary']] == row['binary_sha256']
        assert row['compiler_flags'] == original['compiler_flags']
        compiler = row['runtime']
        assert digest(compiler['path']) == report['pins'][compiler['path']] == compiler['sha256']
        command = [compiler['path']] + original['compiler_flags'] + ['-o', row['binary'], row['source']]
        assert row['build_command'] == command
        build = load(PRIOR / benchmark / row['id'] / 'build.json')
        assert build['command'] == command and build['inherited_common_mutex'] is True
        assert type(build['returncode']) is int and build['returncode'] == 0
        assert type(build['pid']) is int and build['pid'] > 0
        assert not any(build.get(name) for name in ('error', 'error_type', 'cleanup_error', 'cleanup_error_type'))
        assert row['maximum_workers'] == original['maximum_workers'] <= 8
        assert row['algorithm']['strict'] is True and row['algorithm']['native_benchmark_kernel'] is False
        assert row['command'] == [row['binary']] and row['never_chart_eligible'] is True
        assert row['small_validation']['reference_match'] is True
        assert row['build_status'] == 'passed' and row['qualification_status'] == 'bounded-outputs-passed'
    for ordinal, attempt in enumerate(report['attempts'], 1):
        key = (attempt['id'], attempt['case'])
        assert key[0] in profiles and key[1] in cases and key not in observed
        observed.add(key)
        row = profiles[key[0]]
        assert attempt['attempt'] == ordinal and attempt['round'] == 0
        assert attempt['qualified'] is True and attempt['fatal_qualification_fields'] == []
        assert attempt['retryable_environment_only'] is False and attempt['full_workload'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert attempt['full_performance_run'] is False
        assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
        assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
        assert type(attempt['seconds']) in (int, float) and math.isfinite(attempt['seconds']) and attempt['seconds'] > 0
        assert attempt['valid_environment'] is True and attempt['pins_unchanged_at_end'] is True
        assert attempt['power_at_start'] == attempt['power_at_end'] == POWER
        assert attempt['timeout_seconds'] == 300 and attempt['maximum_rss_mib'] == 8192
        assert type(attempt['peak_process_group_rss_kib']) is int and 0 <= attempt['peak_process_group_rss_kib'] <= 8192 * 1024
        assert all(attempt[name] == [] for name in ('remaining_group_members', 'supervisor_remaining_members',
            'monitor_errors', 'resource_monitor_errors', 'competing_compilers', 'power_changes'))
        assert not any(attempt.get(name) for name in ('error', 'resource_violation', 'cleanup_error',
            'supervisor_cleanup', 'supervisor_registration_error', 'sampler_failure', 'supervisor_failure',
            'supervisor_error', 'supervisor_pin_failure', 'monitor_finish_error', 'source_work_error'))
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes']
        assert attempt['validation']['sha256'] == attempt['stdout_sha256']
        assert attempt['command'] == row['command'] and attempt['source_sha256'] == row['source_sha256']
        assert attempt['execution_fingerprint'] == harness.execution_fingerprint(row)
        native = attempt['supervised_native_group']
        assert native['type'] == 'native-group-started' and native['command'] == row['command']
        assert type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
        assert native['inherited_common_mutex'] is True
        raw_path = Path(attempt['result_path'])
        assert raw_path.is_relative_to(PRIOR / benchmark / 'attempts')
        raw = load(raw_path)
        assert all(name in attempt and attempt[name] == value for name, value in raw.items()), 'Saved raw record contradicts ledger'
    assert observed == {(identity, case) for identity in profiles for case in cases}
    return dict(status='verified-existing-bounded-output-proof', benchmark=benchmark,
        profiles=sorted(profiles), bounded_records=len(report['attempts']),
        old_records_relabelled=False, native_cases_reexecuted=0, full_performance_runs=0,
        never_chart_eligible=True)


def reconcile(module, declared, lock_fd):
    state, expected = os.fstat(lock_fd), module.LOCK.stat()
    assert (state.st_dev, state.st_ino) == (expected.st_dev, expected.st_ino)
    fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    harness = module.load_harness()
    pins, benchmarks = {}, {}
    # Preserve the predecessor's actual stop and both compiler diagnostics as
    # lineage. These files are never converted into successful output proof.
    failed = [PRIOR / 'cohort.json', PRIOR / 'pidigits/probes.json']
    for variant in ('compact', 'optimized'):
        directory = PRIOR / 'pidigits' / ('v-pidigits-' + variant + '-input-v2')
        failed += [directory / name for name in ('build.json', 'build.stdout', 'build.stderr', 'program.v')]
    for path in failed:
        pins[str(path)] = module.sha(path)
    predecessor = json.loads((PRIOR / 'pidigits/probes.json').read_text())
    assert predecessor['status'] == 'stopped' and predecessor['profiles'] == predecessor['attempts'] == []
    assert predecessor['active'] is None and predecessor['error_type'] == 'AssertionError'
    for benchmark in RETAINED:
        path = PRIOR / benchmark / 'probes.json'
        report = json.loads(path.read_text())
        harness.PinGuard(report['pins']).verify()
        proof = validate_records(report, declared, benchmark, harness, module.sha,
                                 lambda filename: json.loads(Path(filename).read_text()))
        proof.update(report=str(path), report_sha256=module.sha(path))
        benchmarks[benchmark] = proof
        for filename, digest in report['pins'].items():
            assert filename not in pins or pins[filename] == digest
            pins[filename] = digest
        pins[str(path)] = proof['report_sha256']
        for attempt in report['attempts']:
            filename = attempt['result_path']
            pins[filename] = module.sha(filename)
        for row in report['profiles']:
            for name in ('build.json', 'build.stdout', 'build.stderr'):
                filename = str(PRIOR / benchmark / row['id'] / name)
                pins[filename] = module.sha(filename)
    harness.PinGuard(pins).verify()
    return dict(status='two-existing-benchmark-proofs-reconciled', benchmarks=benchmarks,
        pins=pins, pins_unchanged_at_end=True, records_inserted_in_fresh_ledger=0,
        native_cases_reexecuted=0, full_performance_runs=0, never_chart_eligible=True)
