"""Conservative source/record contracts. Unexecuted candidate; no admission."""
from pathlib import Path

BENCHMARKS = ('regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')
GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received',
    'native_inherited_common_mutex', 'native_exited_normally', 'no_execution_error',
    'no_wrapper_integrity_error', 'no_resource_violation', 'no_resource_monitor_errors',
    'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload',
    'exact_command_and_group', 'complete_output_reference_match',
    'complete_output_bytes_and_hash', 'pins_unchanged',
}
SAFE_FAILURE_GATES = GATES - {
    'native_exited_normally', 'no_execution_error', 'no_resource_violation',
    'complete_output_reference_match', 'complete_output_bytes_and_hash',
}


def profiles(value):
    assert value['status'] == 'source-only-unexecuted'
    assert value['full_processes_authorized'] == 0
    rows = value['profiles']
    expected = {f'prolog-{bench}-{form}' for bench in BENCHMARKS
                for form in ('compact', 'optimized')}
    assert len(rows) == 20 and {row['id'] for row in rows} == expected
    assert len({row['key'] for row in rows}) == 20
    for row in rows:
        bench, form = row['benchmark'], row['variant']
        assert bench in BENCHMARKS and form in ('compact', 'optimized')
        assert row['id'] == f'prolog-{bench}-{form}'
        assert row['key'] == bench + ':' + row['id']
        assert row['language'] == 'prolog' and row['module'] == 'clbg_' + bench
        assert row['source'] == f'sources/{bench}/{form}.pl'
        expected_helpers = ['support/clbg_io.pl']
        if bench in ('revcomp', 'knucleotide'):
            expected_helpers.append('support/clbg_records.pl')
        assert row['support_files'] == expected_helpers
        assert type(row['maximum_workers']) is int and row['maximum_workers'] == 1
        assert row['stdin'] is (bench in ('regexredux', 'revcomp', 'knucleotide'))
        assert row['source_alias_of'] == (f'prolog-{bench}-compact'
                                           if form == 'optimized' else None)
        assert row['canonical_loc'] is None and row['chart_admission'] is False
    return rows


def compiler_inputs(actual, expected_authored, vendor_root, preparation_source):
    """Classify every original source_file/1 result, including unscored tool glue."""
    assert isinstance(actual, list) and len(actual) == len(set(actual))
    expected = {str(Path(path).resolve()) for path in expected_authored}
    observed = set()
    roles = []
    vendor = Path(vendor_root).resolve()
    tool = Path(preparation_source).resolve()
    for raw in actual:
        assert type(raw) is str
        path = Path(raw).resolve()
        if str(path) in expected:
            observed.add(str(path))
            role = 'authored-scored-source'
        elif path == tool:
            role = 'unscored-inventory-tool'
        elif path.is_relative_to(vendor):
            role = 'vendor-runtime-library'
        else:
            raise AssertionError(('Unclassified compiler input', raw))
        roles.append(dict(path=raw, resolved_path=str(path), role=role))
    assert observed == expected, ('Missing compiled authored helper', expected - observed)
    return roles


def runtime_flags(value, expected_home):
    assert value['version_components'] == [10, 0, 2]
    assert all(type(part) is int for part in value['version_components'])
    assert value['bounded'] is False and value['gc'] is True
    assert type(value['architecture']) is str
    assert value['architecture'].startswith(('arm64-', 'aarch64-'))
    assert Path(value['home']).resolve() == Path(expected_home).resolve()
    assert type(value['stack_limit']) is int and 0 < value['stack_limit'] <= 10 * 1024**3
    return value


def complete_attempt(attempt, full):
    checks = attempt.get('qualification_checks')
    assert isinstance(checks, dict) and set(checks) == GATES
    assert all(type(flag) is bool and flag is True for flag in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is full
    for field in ('attempt', 'workload_n', 'sampler_wait_status', 'stdout_bytes', 'exit_status'):
        assert type(attempt[field]) is int and attempt[field] >= 0
    assert attempt['sampler_wait_status'] == attempt['exit_status'] == 0
    assert type(attempt['validation']['bytes']) is int and attempt['validation']['bytes'] >= 0
    assert attempt['validation']['reference_match'] is True
    native = attempt['supervised_native_group']
    assert type(native['pid']) is int and native['pid'] > 0
    assert type(native['pgid']) is int and native['pgid'] == native['pid']
    return attempt


def isolatable(attempt):
    checks = attempt.get('qualification_checks')
    return (isinstance(checks, dict) and set(checks) == GATES and
            all(type(flag) is bool for flag in checks.values()) and
            all(checks[key] is True for key in SAFE_FAILURE_GATES) and
            bool(attempt.get('error')))


def full_work_receipt(value, rows, verify):
    """External actual witness receipt is mandatory; source declarations never suffice."""
    assert value['status'] == 'qualified-required-source-work'
    assert value['chart_admission'] is False
    assert value['language'] == 'prolog'
    assert set(value['profiles']) == {row['id'] for row in rows}
    verify(value['pins'])
    for row in rows:
        actual = value['profiles'][row['id']]
        assert actual['source_sha256'] == row['source_sha256']
        assert actual['binary_sha256'] == row['binary_sha256']
        assert actual['application_artifact_pins'] == row['application_artifact_pins']
        assert actual['status'] == 'passed' and actual['source_review']['status'] == 'passed'
        assert actual['compiler_bound_input_inventory']['status'] == 'qualified-exact-compiled-inputs'
        if row['benchmark'] == 'binarytrees':
            assert actual['allocation_retention_witness']['status'] == 'passed'
            assert actual['original_scored_vm_allocation_review']['status'] == 'passed'
        elif row['benchmark'] == 'knucleotide':
            assert actual['seven_map_capacity_windows_witness']['status'] == 'passed'
        elif row['benchmark'] == 'pidigits':
            assert actual['both_actual_divisions_witness']['status'] == 'passed'
    return value
