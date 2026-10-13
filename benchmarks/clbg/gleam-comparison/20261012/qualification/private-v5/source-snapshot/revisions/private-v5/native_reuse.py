"""Retain fourteen exact Gleam apps /110 cases and the failed revcomp build."""
import copy
import json
from pathlib import Path

import proof_reuse as old
import references

HERE = Path(__file__).resolve().parents[2]
PACKET = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v4'
BUILD = HERE / '.build/private-v4'
PRIOR_PROPOSAL = HERE / 'proposals/private-v4.json'
PRIOR_SHA = '97569becabcf43681ea5c43ff09c8fb206f7f0a936129762166b6c81202dc340'
REUSED_IDS = {'gleam-' + benchmark + '-' + variant for benchmark in (
    'binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot', 'pidigits', 'fasta')
    for variant in ('compact', 'optimized')}
OLD_ID = 'gleam-binarytrees-compact'
PATCHED = {'gleam-revcomp-' + variant for variant in ('compact', 'optimized')}


def validate_sources():
    before = json.loads((HERE / 'revisions/private-v4/profiles.json').read_text())
    after = json.loads((PACKET / 'profiles.json').read_text())
    expected = copy.deepcopy(before)
    for row in expected['profiles']:
        if row['id'] not in PATCHED: continue
        original = row['source']
        corrected = 'revisions/private-v5/sources/revcomp/' + row['variant'] + '.gleam'
        row['source'] = corrected
        row['compiled_source_bundle'] = [corrected if value == original else value for value in row['compiled_source_bundle']]
        text = (HERE / original).read_text()
        assert text.count('<<header:bytes, 10>>') == 1
        assert (HERE / corrected).read_text() == text.replace('<<header:bytes, 10>>', '<<header:bits, 10>>')
    assert after == expected and PATCHED.isdisjoint(REUSED_IDS)
    selected = {name for row in after['profiles'] for name in row['compiled_source_bundle']}
    for name in selected:
        if name.endswith('.gleam'): assert ':bytes' not in (HERE / name).read_text()


def complete_native(attempt):
    old.observations(attempt)
    assert type(attempt['sampler_wait_status']) is int and attempt['sampler_wait_status'] == 0
    assert type(attempt['workload_n']) is int and type(attempt['attempt']) is int
    assert attempt['qualified'] is True
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == old.GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['full_workload'] is False
    native = attempt['supervised_native_group']
    assert native['type'] == 'native-group-started' and native['command'] == attempt['command']
    assert native['inherited_common_mutex'] is True
    assert type(native['pid']) is int and type(native['pgid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
    assert attempt['validation']['reference_match'] is True
    assert type(attempt['stdout_bytes']) is int and attempt['stdout_bytes'] >= 0
    assert type(attempt['validation']['bytes']) is int and attempt['validation']['bytes'] >= 0
    assert (attempt['validation']['bytes'], attempt['validation']['sha256']) == (attempt['stdout_bytes'], attempt['stdout_sha256'])


def complete_attempt(attempt):
    complete_native(attempt)
    assert attempt['full_performance_run'] is False
    assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True


def validate_records(report, inventory):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == PRIOR_SHA and report['full_performance_runs'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    assert report['error_type'] == 'AssertionError' and report['error'] == "('Owned command failed', 'compile', 1)"
    assert report['reused_bounded_cases'] == 4 and report['reused_cases_relabelled_as_attempts'] is False
    assert len(report['profiles']) == 14 and len(report['attempts']) == 106
    rows = {row['id']: row for row in report['profiles']}
    assert len(rows) == 14 and set(rows) == REUSED_IDS
    grouped = {identifier: [] for identifier in rows if identifier != OLD_ID}
    for number, attempt in enumerate(report['attempts'], 1):
        complete_attempt(attempt)
        assert attempt['attempt'] == number and attempt['id'] in grouped
        row = rows[attempt['id']]
        assert attempt['benchmark'] == row['benchmark'] and attempt['key'] == row['key'] and attempt['language'] == 'gleam'
        assert attempt['case'] in inventory[row['benchmark']]
        assert attempt['case'].startswith('n-') and attempt['workload_n'] == int(attempt['case'][2:])
        assert json.loads(attempt['execution_fingerprint']) == old.historical_fingerprint(row)
        assert attempt['source_sha256'] == row['source_sha256'] and attempt['cwd'] == row['cwd']
        assert attempt['runtime_environment'] == row['runtime_environment']
        assert attempt['command'] == [part.replace('{n}', str(attempt['workload_n'])) for part in row['command']]
        assert attempt['case'] not in {value['case'] for value in grouped[row['id']]}
        grouped[row['id']].append(copy.deepcopy(attempt))
    for identifier, records in grouped.items():
        assert [row['case'] for row in records] == inventory[rows[identifier]['benchmark']]
    return rows, grouped


def validate_receipts(sampled, enriched, attempt):
    old.observations(sampled); old.observations(enriched)
    complete_native(enriched)
    assert sampled['full_workload'] is enriched['full_workload'] is attempt['full_workload'] is False
    for value in (sampled, enriched, attempt):
        assert type(value['stdout_bytes']) is int and value['stdout_bytes'] >= 0
        assert type(value['validation']['bytes']) is int and value['validation']['bytes'] >= 0
        assert value['validation']['reference_match'] is True
    assert all(type(value['workload_n']) is int for value in (sampled, enriched, attempt))
    assert sampled['workload_n'] == enriched['workload_n'] == attempt['workload_n']
    for key in old.OBSERVATIONS + ('command', 'cwd', 'source_sha256', 'execution_fingerprint', 'runtime_environment',
                                 'stdout_bytes', 'stdout_sha256', 'validation'):
        assert sampled[key] == enriched[key] == attempt[key]
    for key in ('attempt', 'qualified', 'qualification_checks', 'supervised_native_group', 'sampler_wait_status'):
        assert enriched[key] == attempt[key]


def reconstruct(preparation, service, guard, bootstrap=None):
    validate_sources()
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    proposal = json.loads(PRIOR_PROPOSAL.read_text()); guard.verify(proposal['pins'])
    reuse_path = Path(proposal['prior_app_reuse'])
    assert preparation.sha(reuse_path) == proposal['prior_app_reuse_sha256']
    retained_bootstrap = json.loads(reuse_path.read_text())
    old.validate_packet(retained_bootstrap, preparation, service, guard)
    if bootstrap is not None: assert bootstrap == retained_bootstrap
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    assert proposal['case_inventory'] == preparation.preflight_inputs(references)
    rows, grouped = validate_records(report, proposal['case_inventory'])
    pins = service.merge(proposal['pins'], retained_bootstrap['pins'], service.pin_tree(PRIOR), service.pin_tree(BUILD),
                         {str(PRIOR_PROPOSAL): PRIOR_SHA, str(report_path): preparation.sha(report_path)})
    compiler = Path(retained_bootstrap['toolchain']['compiler']); libraries = Path(retained_bootstrap['toolchain']['libraries'])
    expected_old = dict(retained_bootstrap['profile'], qualification_origin='exact-unchanged-private-v1-app-and-four-records',
                        new_native_cases=0, prior_bounded_records=retained_bootstrap['prior_records'])
    assert rows[OLD_ID] == expected_old
    harness = guard.load_harness()
    profiles = {OLD_ID: copy.deepcopy(expected_old)}
    for identifier, records in grouped.items():
        row = rows[identifier]
        declaration = next(value for value in proposal['profiles'] if value['id'] == identifier)
        assert all(row[key] == value for key, value in declaration.items() if key != 'source')
        assert row['compiler_version'] == preparation.GLEAM_VERSION and row['runtime_version'] == '29.1.1'
        assert row['measured_architecture'] == 'arm64' and row['backend'] == 'BEAM load-time JIT'
        assert json.loads(harness.execution_fingerprint(row)) == old.historical_fingerprint(row)
        application = BUILD / 'applications' / identifier; package = BUILD / 'packages' / identifier
        assert row['command'] == preparation.runtime_command(application, libraries)
        assert preparation.sha(row['command'][0]) == row['binary_sha256']
        artifact_pins = service.pin_tree(application)
        assert row['compiled_artifacts'] == artifact_pins
        inventory_path = Path(row['compiled_input_inventory'])
        assert preparation.sha(inventory_path) == row['compiled_input_inventory_sha256']
        inventory = json.loads(inventory_path.read_text())
        command = preparation.compile_command(compiler, package, application, libraries)
        assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == identifier
        assert inventory['compiler'] == str(compiler) and inventory['compiler_sha256'] == preparation.sha(compiler)
        assert inventory['build_command'] == command
        assert inventory['source_bundle'] == row['source_bundle'] and inventory['compiled_artifacts'] == artifact_pins
        assert inventory['runtime_classpath'] == row['runtime_classpath'] == [str(application / 'ebin'), str(libraries / 'gleam_stdlib/ebin')]
        receipt = json.loads((PRIOR / 'builds' / identifier / 'compile.json').read_text())
        assert receipt['command'] == command and receipt['inherited_common_mutex'] is True
        assert type(receipt['exit_status']) is type(receipt['final_exit_status']) is int and receipt['exit_status'] == receipt['final_exit_status'] == 0
        assert not any(receipt.get(key) for key in ('error', 'cleanup_error', 'cleanup_errors', 'output_drain_error', 'remaining_group_members'))
        assert len(row['source_bundle']) == len(declaration['compiled_source_bundle'])
        assert row['source_bundle'][0]['path'] == row['source'] and row['source_bundle'][0]['sha256'] == row['source_sha256']
        for item, original in zip(row['source_bundle'], declaration['compiled_source_bundle']):
            assert item['origin'] == str(PRIOR / 'source-snapshot' / original)
            assert preparation.sha(item['path']) == item['sha256'] == preparation.sha(item['origin']) == preparation.sha(HERE / original)
        support = service.merge({item['path']: item['sha256'] for item in row['source_bundle'][1:]}, artifact_pins,
                                service.pin_tree(libraries), {str(preparation.ENGINE): preparation.sha(preparation.ENGINE),
                                                             str(preparation.ERLEXEC): preparation.sha(preparation.ERLEXEC)})
        assert row['support'] == support; guard.verify(support)
        retained_records = []
        for attempt in records:
            case = attempt['case']; directory = PRIOR / identifier / case
            expected = getattr(references, row['benchmark'])(attempt['workload_n'])
            actual = (directory / 'actual.stdout').read_bytes()
            assert (directory / 'expected.stdout').read_bytes() == expected
            if row['benchmark'] == 'nbody':
                harness.compact.common.validate(actual.decode(), [float(value) for value in expected.splitlines()])
            else:
                assert actual == expected
            assert preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
            raw = PRIOR / 'attempts' / f"{attempt['attempt']:03d}-{identifier}.json"; supervisor = raw.with_suffix('.supervisor.json')
            assert attempt['result_path'] == str(raw) and attempt['supervisor_result_path'] == str(supervisor)
            validate_receipts(json.loads(raw.read_text()), json.loads(supervisor.read_text()), attempt)
            retained_records.append(dict(case=case, prior_attempt=attempt['attempt'], raw_result=str(raw),
                supervisor_result=str(supervisor), stdout_sha256=attempt['stdout_sha256'], origin='retained-private-v4'))
        profiles[identifier] = dict(row, qualification_origin='exact-unchanged-private-v4-app-and-complete-records',
                                    new_native_cases=0, prior_bounded_records=retained_records)
    failed = PRIOR / 'builds/gleam-revcomp-compact'
    failed_receipt = json.loads((failed / 'compile.json').read_text())
    failed_command = preparation.compile_command(compiler, BUILD / 'packages/gleam-revcomp-compact',
                                                BUILD / 'applications/gleam-revcomp-compact', libraries)
    assert failed_receipt['command'] == failed_command and failed_receipt['inherited_common_mutex'] is True
    assert type(failed_receipt['exit_status']) is type(failed_receipt['final_exit_status']) is int
    assert failed_receipt['exit_status'] == failed_receipt['final_exit_status'] == 1
    assert failed_receipt['error_type'] == 'AssertionError' and failed_receipt['error'] == "('Owned command failed', 'compile', 1)"
    assert not any(failed_receipt.get(key) for key in ('cleanup_error', 'cleanup_errors', 'output_drain_error', 'remaining_group_members'))
    stderr = (failed / 'compile.stderr').read_text()
    assert 'Invalid bit array segment' in stderr and '<<header:bytes, 10>>' in stderr and 'only allowed in BitArray patterns' in stderr
    failed_source = BUILD / 'packages/gleam-revcomp-compact/src/benchmark.gleam'
    assert failed_source.read_bytes() == (HERE / 'sources/revcomp/compact.gleam').read_bytes()
    assert len(profiles) == 14 and sum(len(row['prior_bounded_records']) for row in profiles.values()) == 110
    guard.verify(pins)
    return dict(status='verified-fourteen-exact-Gleam-apps-110-cases', profiles=profiles, reused_bounded_cases=110,
        reused_profile_count=14, old_cases_relabelled_as_new_attempts=False, native_cases_reexecuted=0,
        prior_report=str(report_path), prior_report_sha256=pins[str(report_path)], prior_proposal_sha256=PRIOR_SHA,
        preserved_failed_build=dict(receipt=str(failed / 'compile.json'), stderr=str(failed / 'compile.stderr'), source=str(failed_source)),
        toolchain=retained_bootstrap['toolchain'], actual_toolchain=retained_bootstrap['actual_toolchain'], pins=pins,
        recorded_in_completed_preparation_pins=False, newly_captured_preserved_native_evidence=True,
        full_performance_runs=0, chart_admission=False)


def validate_packet(packet, preparation, service, guard, bootstrap=None):
    guard.verify(packet['pins']); assert packet == reconstruct(preparation, service, guard, bootstrap)
    return packet
