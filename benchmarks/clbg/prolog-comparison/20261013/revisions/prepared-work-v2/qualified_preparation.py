"""Read-only reuse of the actual20/156 preparation; no native entry point."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
PREP = H / 'revisions/prepare-success-halt-v1/guarded_prepare.py'
PREP_SHA = '8d7cefd54b0342b563aec6131e8cdb443203309c0719b804a26d991e0c227ecb'
OLD = H / 'qualification/prepare-explicit-goal-rest-v1'
OWNER = C / 'owned-jobs/prolog-remaining155-preparation-v1'
ROOT = C / 'jobs/prolog-remaining155-closed-review-v1/receipt.json'
ROOT_SHA = '139d9124d3df942bb3170112b3841c33b0191580a002f974835a627651be9bc8'
REPORT_SHA = 'cab6ea7f3bd8a237f8f83833ed9b9cc293bbe1878cc8a894d74fdcf41f7ffbff'
BODY_SHA = '716ba1503f06629304bba8932569d8418d214794a41d0b0395dab60bff20029d'
OWNER_SHA = '21d10035fea80c16dd4514f762e8a92f79af806662b2aed19f133c339906842f'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
REST = H / 'revisions/prepare-explicit-goal-rest-v1'
REST_SHA = 'f74bb1604cb1559305b8b435af26768e3c5c462afd000c3deae4939632ed6a59'
REST_NOTE_SHA = '4fc9e5574840c2867b5bd68e0998b5e363a5936b5eff4c5f13c70e63b7fcbd40'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, expected, pins):
    # Keep each recorded lookup path as well as the separately recorded real
    # path: resolving here would silently discard original symlink-route pins.
    path = Path(path).absolute()
    assert sha(path) == expected and (str(path) not in pins or pins[str(path)] == expected), path
    pins[str(path)] = expected
    return path


def read(path, expected, pins):
    return json.loads(bind(path, expected, pins).read_bytes())


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed actual prerequisite', path)


def cold(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules
    raw = bind(path, pins[str(path)], pins).read_bytes()
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    bind(path, pins[str(path)], pins)
    return module


def leases():
    return [OWNER, C / 'owned-jobs/prolog-explicit-goal-first-case-v1',
            C / 'owned-jobs/prolog-runtime-success-halt-v1']


def load(pins):
    """Caller holds common and the three actual closed private leases."""
    root = read(ROOT, ROOT_SHA, pins)
    assert root['status'] == 'closed-qualified-all20-Prolog-bounded-preparation-awaiting-work-and-LOC'
    assert root['verified_pins'] == 14611 and root['report_sha256'] == REPORT_SHA
    assert root['body_terminal_sha256'] == BODY_SHA and root['durable_terminal_sha256'] == OWNER_SHA
    assert root['profiles'] == 20 and root['total_bounded_cases'] == 156 and root['qualification_checks_per_case'] == 20
    assert root['new_bounded_cases'] == 155 and root['new_build_profiles'] == 19
    assert root['successful_compiler_commands_reused'] == 3 and root['successful_QLF_artifacts_reused'] == 2
    assert all(root[key] == 0 for key in ('successful_case_reexecutions', 'fixture_reexecutions', 'benchmark_runs'))
    assert root['source_work_qualified'] is root['chart_admission'] is False and root['canonical_loc'] is None
    done = read(OWNER / 'terminal.json', OWNER_SHA, pins)
    request, start, worker = [read(OWNER / name, sha(OWNER / name), pins)
        for name in ('request.json', 'command-started.json', 'worker-started.json')]
    argv = [PYTHON, '-I', '-S', '-B', '-u', str(REST / 'prepare.py'),
        '--worker-sha256', REST_SHA, '--review-sha256', REST_NOTE_SHA,
        '--closed-review-sha256', '785fb78443a3a46384533b522ab124b34c2ae87a7e11da4da22a083289099b4e',
        '--success-review-sha256', '234973fa0437b6b344d8ebe19ad07a4518d78a09086593579e06a70cc3456f22']
    assert request['command'] == start['command'] == worker['command'] == argv
    assert request['cwd'] == start['cwd'] == worker['cwd'] == str(C.parents[2])
    assert request['common_mutex_owned_by_target_command'] is True
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert done['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(done['exit_code']) is int and done['exit_code'] == 0 and done['stop_signals'] == []
    assert done['actual_command_started'] is True and done['command_identity_observation_missing'] is False
    assert done['qualification_granted'] is done['automatic_retry'] is False
    assert not any(done.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
    assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid'] == 36320
    assert start['command_identity_observation_missing'] is False
    assert worker['worker_identity'] == done['worker_identity'] and done['worker_identity']['pid'] == 36316
    listing = done['command_identity']['listing'].split(None, 7)
    assert listing[:2] == ['36320', '36316'] and listing[7].split() == argv
    bind(REST / 'prepare.py', REST_SHA, pins)
    bind(REST / 'REVIEW.md', REST_NOTE_SHA, pins)
    old = read(OLD / 'report.json', REPORT_SHA, pins)
    body = read(OLD / 'terminal-pin.json', BODY_SHA, pins)
    assert body['status'] == old['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
    assert body['report'] == str(OLD / 'report.json') and body['report_sha256'] == REPORT_SHA
    assert body['inherited_common_mutex'] is old['terminal_evidence_complete'] is True
    assert body['partial_evidence_pins'] == old['pins']
    assert body['chart_admission'] is old['chart_admission'] is False and body['full_processes_authorized'] == 0
    assert not any(old.get(key) for key in ('error', 'error_type', 'terminal_artifact_errors'))
    for path, digest in old['pins'].items():
        bind(path, digest, pins)
    bind(PREP, PREP_SHA, pins)
    prep = cold(PREP, 'prolog_prepared_actual_original_controller', pins)
    cfg, runtime_pins, binary, home = prep.runtime_config(argparse.Namespace(
        runtime_config=prep.RUNTIME, runtime_config_sha256=prep.RUNTIME_SHA))
    for path, digest in runtime_pins.items():
        bind(path, digest, pins)
    service = cold(prep.SERVICE, 'prolog_prepared_actual_owned_service', pins)
    references = cold(prep.REFERENCES, 'prolog_prepared_actual_references', pins)
    guard = cold(service.GUARD, 'prolog_prepared_actual_guard', pins)
    contracts = cold(PREP.parent / 'contracts.py', 'prolog_prepared_actual_contracts', pins)
    harness = guard.load_harness()
    for module in (harness, harness.compact, harness.compact.common):
        path = Path(module.__file__).resolve(strict=True)
        bind(path, pins[str(path)], pins)
    original, merged, derivation = prep.original_proposal(pins)
    for path, digest in merged.items():
        bind(path, digest, pins)
    declared = contracts.profiles(read(PREP.parent / 'profiles.json', pins[str(PREP.parent / 'profiles.json')], pins))
    assert declared == original['profiles']
    assert set(old['profiles']) == {row['id'] for row in declared} and len(old['profiles']) == 20
    assert old['all20_bounded_outputs_qualified'] is True and old['total_bounded_cases'] == 156
    assert old['new_bounded_cases'] == len(old['attempts']) == 155 and len(old['reused_attempts']) == 1
    assert old['new_source_profiles_built'] == 19 and old['retained_contract_fixture_count'] == 6
    assert old['retained_ownership_fixture_count'] == 11 and old['canonical_loc'] is None
    assert old['source_work_qualified'] is False and old['full_processes_authorized'] == 0
    generic = read(prep.SERVICE_PROPOSAL, pins[str(prep.SERVICE_PROPOSAL)], pins)
    fixture = read(generic['actual_mock_fixture_receipt'], generic['actual_mock_fixture_receipt_sha256'], pins)
    assert fixture['status'] == 'passed' and fixture['inventory'] == 'ownership' and fixture['successful'] is True
    assert fixture['expected_test_ids'] == prep.OWNERSHIP_IDS
    assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
    assert fixture['tests_run'] == fixture['discovered_tests'] == 11
    assert len(fixture['actual_test_ids']) == len(set(fixture['actual_test_ids'])) == 11
    assert set(fixture['actual_test_ids']) == set(prep.OWNERSHIP_IDS)
    assert all(fixture[key] == [] for key in ('skipped', 'failures', 'errors', 'expected_failures', 'unexpected_successes'))
    for declaration in declared:
        row = old['profiles'][declaration['id']]
        assert row['benchmark'] == declaration['benchmark'] and row['variant'] == declaration['variant']
        assert row['module'] == declaration['module'] and row['bounded_output_qualified'] is True
        assert row['source_work_qualified'] is row['chart_admission'] is False and row['canonical_loc'] is None
        source = bind(row['measured_source'], row['source_sha256'], pins)
        bundle = row['source_bundle']
        assert [item['original_path'] for item in bundle] == [str(PREP.parent / name)
            for name in declaration['support_files'] + [declaration['source']]]
        assert [item['role'] for item in bundle] == ['counted-authored-helper'] * len(declaration['support_files']) + ['main-source']
        assert bundle[-1]['path'] == str(source) and len({item['path'] for item in bundle}) == len(bundle)
        for item in bundle:
            copied = bind(item['path'], item['sha256'], pins)
            origin = bind(item['original_path'], item['sha256'], pins)
            assert copied.read_bytes() == origin.read_bytes()
        artifacts = [Path(item['path']).with_suffix('.qlf') for item in bundle]
        assert row['application_artifact_pins'] == {str(path): pins[str(path)] for path in artifacts}
        expected_command = prep.prefix(binary)
        for artifact in artifacts[:-1]:
            expected_command += ['-l', str(artifact)]
        expected_command += ['-s', str(artifacts[-1]), '-g', 'clbg_io:run(' + row['module'] + ':main)']
        if not row['stdin']:
            expected_command += ['--', '{n}']
        assert row['command'] == expected_command and row['binary'] == str(binary)
        assert row['binary_sha256'] == cfg['binary_sha256'] and row['runtime_environment'] == cfg['environment']
        assert row['cwd'] == str(source.parents[2])
        for item in row['compile_receipts']:
            native = read(item['receipt'], item['receipt_sha256'], pins)
            prep.owned_receipt(native, item['command'])
            inventory = read(item['input_inventory_receipt'], item['input_inventory_receipt_sha256'], pins)
            contracts.runtime_flags(inventory['runtime'], home)
            prep.compiler_libraries(inventory, cfg)
        compiler = row['compiler_bound_input_inventory']
        assert compiler['status'] == 'qualified-exact-compiled-inputs'
        actual = read(compiler['receipt'], compiler['receipt_sha256'], pins)
        contracts.runtime_flags(actual['runtime'], home)
        prep.compiler_libraries(actual, cfg)
        assert contracts.compiler_inputs(actual['loaded_sources'], [Path(item['path']) for item in bundle],
            home, PREP.parent / 'tools/prepare_qlf.pl') == compiler['inputs']
        bind(compiler['original_scored_wam'], compiler['original_scored_wam_sha256'], pins)
    attempts = old['attempts'] + old['reused_attempts']
    actual_attempts = {(item['id'], item['case']): item for item in attempts}
    assert len(actual_attempts) == len(attempts) == 156
    for item in attempts:
        contracts.complete_attempt(item, False)
        prep.native_raw_receipts(item)
        assert item['stderr'] == '' and item['qualified'] is True
    cases = prep.bounded_cases(service, references)
    expected_cases = {(row['id'], label) for row in declared for label, _, _ in cases[row['benchmark']]}
    results = old['case_results']
    assert len(results) == len(expected_cases) == 156
    assert {(item['id'], item['case']) for item in results} == expected_cases
    assert len({(item['id'], item['actual_case']) for item in results}) == 156
    for result in results:
        attempt = actual_attempts[(result['id'], result['actual_case'])]
        assert result['result_path'] == attempt['result_path'] and result['supervisor_result_path'] == attempt['supervisor_result_path']
    verify(pins)
    return dict(prep=prep, service=service, references=references, guard=guard, harness=harness,
        contracts=contracts, config=cfg, binary=binary, home=home, report=old, derivation=derivation)
