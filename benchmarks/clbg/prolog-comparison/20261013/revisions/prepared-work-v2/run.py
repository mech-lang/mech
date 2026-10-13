"""Original six work fixtures, then thirty distinct instrumented small probes."""
import argparse
import copy
from contextlib import ExitStack
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys
import traceback

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = ('run.py', 'qualified_preparation.py', 'fixture_runner.py',
    'instrument_sources.py', 'work_proofs.py', 'test_work_proofs.py', 'observer.pl', 'REVIEW.md')
FIXTURES = H / 'work-qualification/prepared-work-v2-fixtures-v1'
WORK = H / 'work-qualification/prepared-work-v2-probes-v1'
TREE_SIZES = (0, 4, 7, 9)
PI_SIZES = (0, 1, 9, 10, 11, 27, 97)
MAP_INPUTS = (
    ('missing-three', b'>ONE\nACGT\n', ''),
    ('one-base', b'>THREE\nA\n', 'A'),
    ('overlap-and-tail', b'>THREE\nGGTATTTTAATTTATAGTACGTGGT\n>FOUR\nAAAA\n', 'GGTATTTTAATTTATAGTACGTGGT'),
    ('arbitrary-width18-keys', b'>THREE\nACGTACGTACGTACGTACGTACGTACGT\n', 'ACGTACGTACGTACGTACGTACGTACGT'),
)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert sha(path) == digest and (str(path) not in pins or pins[str(path)] == digest), path
    pins[str(path)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


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


def packet(args, pins):
    path = Path(args.packet).resolve(strict=True)
    assert not path.stat().st_mode & 0o222
    value = read(path, args.packet_sha256, pins)
    expected = {str((P / name).resolve()) for name in FILES} | {str(Path(sys.executable).resolve())}
    assert set(value) == {'schema', 'pins'} and value['schema'] == 'frozen-prolog-prepared-work-v2'
    assert set(value['pins']) == expected
    for filename, digest in value['pins'].items():
        if filename != str(Path(sys.executable).resolve()):
            assert not Path(filename).stat().st_mode & 0o222
        bind(filename, digest, pins)


def closed_fixtures(args, pins, authority, dependencies):
    owner = Path(args.fixture_owner).resolve(strict=True)
    assert owner.parent == C / 'owned-jobs'
    lease = dependencies.enter_context((owner / 'owner.lock').open('rb'))
    fcntl.flock(lease, fcntl.LOCK_SH | fcntl.LOCK_NB)
    held, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    done = read(owner / 'terminal.json', args.fixture_terminal_sha256, pins)
    records = [read(owner / name, sha(owner / name), pins)
        for name in ('request.json', 'command-started.json', 'worker-started.json')]
    request, start, worker = records
    expected = [authority.PYTHON, '-I', '-S', '-B', '-u', str(P / 'run.py'),
        '--fixtures', '--packet', str(Path(args.packet).resolve()), '--packet-sha256', args.packet_sha256]
    assert request['command'] == start['command'] == worker['command'] == expected
    assert request['cwd'] == start['cwd'] == worker['cwd'] == str(C.parents[2])
    assert request['common_mutex_owned_by_target_command'] is True
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert done['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(done['exit_code']) is int and done['exit_code'] == 0 and done['stop_signals'] == []
    assert done['actual_command_started'] is True and done['command_identity_observation_missing'] is False
    assert done['qualification_granted'] is done['automatic_retry'] is False
    assert not any(done.get(key) for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
    assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid']
    assert start['command_identity_observation_missing'] is False
    assert worker['worker_identity'] == done['worker_identity']
    command_pid, worker_pid = done['command_pid'], done['worker_identity']['pid']
    assert type(command_pid) is type(worker_pid) is int and command_pid > 0 and worker_pid > 0
    listing = done['command_identity']['listing'].split(None, 7)
    assert listing[:2] == [str(command_pid), str(worker_pid)] and listing[7].split() == expected
    report = read(FIXTURES / 'report.json', args.fixture_report_sha256, pins)
    body = read(FIXTURES / 'terminal-pin.json', args.fixture_body_sha256, pins)
    assert body['status'] == report['status'] == 'six-work-fixtures-passed-awaiting-thirty-probes'
    assert report['terminal_evidence_complete'] is body['inherited_common_mutex'] is True
    assert body['report'] == str(FIXTURES / 'report.json') and body['report_sha256'] == args.fixture_report_sha256
    assert body['partial_evidence_pins'] == report['pins']
    assert not any(report.get(key) for key in ('error', 'error_type', 'terminal_artifact_errors'))
    assert report['new_native_commands'] == report['work_probe_processes'] == 0
    for path, digest in report['pins'].items():
        bind(path, digest, pins)
    fixture = read(FIXTURES / 'fixture-receipt.json', pins[str(FIXTURES / 'fixture-receipt.json')], pins)
    runner = cold(P / 'fixture_runner.py', 'prolog_closed_six_work_fixture_contract', pins)
    assert fixture['status'] == 'passed' and fixture['execution_completed'] is fixture['inherited_common_mutex'] is True
    assert fixture['expected_test_ids'] == runner.IDS
    assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
    assert fixture['tests_run'] == fixture['discovered_tests'] == 6
    for key in ('actual_test_ids', 'discovered_test_ids'):
        assert len(fixture[key]) == len(set(fixture[key])) == 6 and set(fixture[key]) == set(runner.IDS)
    assert all(fixture[key] == [] for key in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
    return fixture


def build_observed(row, output, original, modules, pins, lock_fd, report):
    prep, service, guard, harness = [modules[key] for key in ('prep', 'service', 'guard', 'harness')]
    cfg, binary, home, contracts = [modules[key] for key in ('config', 'binary', 'home', 'contracts')]
    build = output / 'builds' / row['id']
    root = build / 'input'
    main_original = Path(original['measured_source'])
    main_relative = main_original.relative_to(Path(original['cwd']))
    main = root / main_relative
    authored, bundle, compiler_receipts = [], [], []
    for item in original['source_bundle'][:-1]:
        source = bind(item['path'], item['sha256'], pins)
        target = root / source.relative_to(Path(original['cwd']))
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(source.read_bytes())
        assert sha(target) == item['sha256']
        pins[str(target)] = item['sha256']
        authored.append(target)
        bundle.append(dict(path=str(target), sha256=item['sha256'], role='unchanged-authored-helper', origin=str(source)))
    observer = root / 'support/clbg_observer.pl'
    observer.parent.mkdir(parents=True, exist_ok=True)
    with observer.open('xb') as stream:
        stream.write((P / 'observer.pl').read_bytes())
    assert sha(observer) == pins[str(P / 'observer.pl')]
    pins[str(observer)] = sha(observer)
    authored.append(observer)
    bundle.append(dict(path=str(observer), sha256=sha(observer), role='unscored-observer', origin=str(P / 'observer.pl')))
    source = bind(main_original, original['source_sha256'], pins).read_bytes()
    text, edits = modules['instrument'].transform(row['benchmark'], source.decode('utf-8'), observer)
    main.parent.mkdir(parents=True, exist_ok=True)
    with main.open('xb') as stream:
        stream.write(text.encode('utf-8'))
    pins[str(main)] = sha(main)
    authored.append(main)
    bundle.append(dict(path=str(main), sha256=sha(main), role='unscored-instrumented-main', origin=str(main_original)))
    save(build / 'source-derivation.json', dict(original_main=str(main_original), original_sha256=original['source_sha256'],
        instrumented_main=str(main), instrumented_sha256=sha(main), reverse_projection_exact=True,
        exact_insertions=edits, observer=str(observer), observer_sha256=sha(observer), original_scored_sources_unchanged=True))
    environment = dict(harness.compact.ENV) | cfg['environment']
    for target in authored:
        receipt, wam = build / (target.stem + '-compile.json'), build / (target.stem + '-compile.wam')
        command = prep.compiler_command(binary, 'compile', target, receipt, wam, root)
        result = service.owned_command(guard, harness, command, environment, build, lock_fd, 120, target.stem + '-compile-command')
        report['new_native_commands'] += 1
        prep.owned_receipt(result['detail'], command)
        actual = json.loads(receipt.read_bytes())
        contracts.runtime_flags(actual['runtime'], home)
        prep.compiler_libraries(actual, cfg)
        assert target.with_suffix('.qlf').is_file()
        compiler_receipts.append(dict(source=str(target), command=command,
            owned_receipt=str(build / (target.stem + '-compile-command.json')), owned_receipt_sha256=sha(build / (target.stem + '-compile-command.json')),
            inventory_receipt=str(receipt), inventory_receipt_sha256=sha(receipt)))
    artifacts = [path.with_suffix('.qlf') for path in authored]
    inventory, wam = build / 'loaded-inputs.json', build / 'instrumented.wam'
    command = prep.compiler_command(binary, 'inspect', artifacts[-1], inventory, wam, root, artifacts[:-1])
    result = service.owned_command(guard, harness, command, environment, build, lock_fd, 120, 'inspect-command')
    report['new_native_commands'] += 1
    prep.owned_receipt(result['detail'], command)
    actual = json.loads(inventory.read_bytes())
    contracts.runtime_flags(actual['runtime'], home)
    prep.compiler_libraries(actual, cfg)
    roles = contracts.compiler_inputs(actual['loaded_sources'], authored, home, prep.PACKET / 'tools/prepare_qlf.pl')
    for item in roles:
        filename = item['resolved_path']
        if item['role'] == 'vendor-runtime-library':
            bind(filename, cfg['pins'][filename], pins)
        else:
            bind(filename, pins[filename], pins)
    for path in build.rglob('*'):
        if path.is_file():
            assert str(path) not in pins or pins[str(path)] == sha(path)
            pins[str(path)] = sha(path)
    invocation = prep.prefix(binary)
    for artifact in artifacts[:-1]:
        invocation += ['-l', str(artifact)]
    invocation += ['-s', str(artifacts[-1]), '-g', 'clbg_io:run(' + row['module'] + ':main)']
    if not row['stdin']:
        invocation += ['--', '{n}']
    row.update(measured_source=str(main), source_sha256=sha(main), source_bundle=bundle,
        binary=str(binary), binary_sha256=cfg['binary_sha256'], command=invocation,
        cwd=str(root), runtime_environment=cfg['environment'],
        application_artifact_pins={str(path): sha(path) for path in artifacts},
        observer_compiler_receipts=compiler_receipts, observer_loaded_sources=roles,
        instrumented_wam=str(wam), instrumented_wam_sha256=sha(wam),
        original_scored_profile=copy.deepcopy(original), instrumented_only=True,
        source_work_qualified=False, canonical_loc=None, chart_admission=False)
    return row


def probes(output, modules, pins, lock_fd, report):
    prep, service, guard, harness = [modules[key] for key in ('prep', 'service', 'guard', 'harness')]
    modules['instrument'] = cold(P / 'instrument_sources.py', 'instrument_sources', pins)
    proofs = cold(P / 'work_proofs.py', 'work_proofs', pins)
    guard.OUT, guard.REPORT = output, output / 'report.json'
    harness.install_interrupt_handlers()
    harness.compact.wait_for_environment(harness.REQUIRED)
    rows = modules['report']['profiles']
    expected_cases = []
    for bench in ('binarytrees', 'pidigits', 'knucleotide'):
        for form in ('compact', 'optimized'):
            identifier = 'prolog-' + bench + '-' + form
            original = rows[identifier]
            row = build_observed(copy.deepcopy(original), output, original, modules, pins, lock_fd, report)
            report['instrumented_profiles'][identifier] = row
            if bench == 'binarytrees':
                cases = [('n-' + str(n), n, None, None) for n in TREE_SIZES]
            elif bench == 'pidigits':
                cases = [('n-' + str(n), n, None, None) for n in PI_SIZES]
            else:
                cases = [(label, 0, data, sequence) for label, data, sequence in MAP_INPUTS]
            for label, n, data, sequence in cases:
                identity = (identifier, label)
                expected_cases.append(identity)
                trace = output / 'native-traces' / identifier / (label + '.jsonl')
                trace.parent.mkdir(parents=True, exist_ok=True)
                assert not trace.exists()
                current = copy.deepcopy(row)
                current['runtime_environment'] = dict(row['runtime_environment'], MECH_PROLOG_TRACE=str(trace))
                oracle = getattr(modules['references'], bench)
                expected = oracle(data if data is not None else n)
                def reconcile(directory):
                    events = proofs.events(trace)
                    if bench == 'binarytrees':
                        proof = proofs.trees(events, n)
                    elif bench == 'knucleotide':
                        assert modules['references'].nucleotide_maps(data)[0] == sequence
                        proof = proofs.maps(events, sequence)
                    else:
                        digits = ''.join(line.split(b'\t')[0].decode().replace(' ', '') for line in expected.splitlines())
                        proof = proofs.pi(events, n, digits)
                    assert proof['status'] == 'passed'
                    pins[str(trace)] = sha(trace)
                    save(directory / 'source-work-proof.json', dict(native_trace=str(trace), native_trace_sha256=sha(trace),
                        original_profile=identifier, original_source_sha256=original['source_sha256'],
                        original_scored_wam=original['compiler_bound_input_inventory']['original_scored_wam'],
                        original_scored_wam_sha256=original['compiler_bound_input_inventory']['original_scored_wam_sha256'],
                        observed_program_sha256=current['source_sha256'], proof=proof))
                    return proof
                attempt = service.probe(guard, harness, current, n, data, expected, pins,
                    output / 'bounded-output', lock_fd, report, label, trace=reconcile)
                modules['contracts'].complete_attempt(attempt, False)
                prep.native_raw_receipts(attempt)
                assert attempt['stderr'] == '' and attempt['source_work_proof']['status'] == 'passed'
                report['work_probe_processes'] += 1
                report['new_native_commands'] += 1
                report['case_results'].append(dict(id=identifier, case=label, native_trace=str(trace),
                    native_trace_sha256=sha(trace), result_path=attempt['result_path'], supervisor_result_path=attempt['supervisor_result_path']))
                guard.save(guard.REPORT, report)
    assert len(expected_cases) == len(set(expected_cases)) == 30
    assert len(report['case_results']) == len(report['attempts']) == report['work_probe_processes'] == 30
    assert {(row['id'], row['case']) for row in report['case_results']} == set(expected_cases)
    assert len(report['instrumented_profiles']) == 6 and report['new_native_commands'] == 56
    report.update(status='thirty-instrumented-work-probes-passed-awaiting-scored-source-and-WAM-review',
        actual_tree_map_pi_observations_reconciled=True, original_scored_work_receipt_qualified=False)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--fixtures', action='store_true')
    mode.add_argument('--work-probes', action='store_true')
    parser.add_argument('--packet', required=True)
    parser.add_argument('--packet-sha256', required=True)
    for name in ('fixture-owner', 'fixture-terminal-sha256', 'fixture-report-sha256', 'fixture-body-sha256'):
        parser.add_argument('--' + name)
    args = parser.parse_args()
    assert sys.dont_write_bytecode
    assert re.fullmatch('[a-f0-9]{64}', args.packet_sha256)
    assert all(bool(getattr(args, key)) is args.work_probes for key in
        ('fixture_owner', 'fixture_terminal_sha256', 'fixture_report_sha256', 'fixture_body_sha256'))
    def interrupt(signum, frame):
        raise KeyboardInterrupt('Prolog work gate interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupt)
    print('WAIT common mutex: Prolog six work fixtures or thirty work probes', flush=True)
    with LOCK.open('r+') as common, ExitStack() as dependencies:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        output = FIXTURES if args.fixtures else WORK
        output.mkdir(parents=True, exist_ok=False)
        pins, modules, failure = {}, None, None
        report = dict(status='running-Prolog-prepared-work-gate', chart_admission=False,
            source_work_qualified=False, canonical_loc=None, full_processes_authorized=0,
            attempts=[], active=None, instrumented_profiles={}, case_results=[], new_native_commands=0,
            work_probe_processes=0, successful_preparation_cases_reexecuted=0, original_scored_profiles_recompiled=0,
            old_contract_fixture_reexecutions=0, old_ownership_fixture_reexecutions=0)
        signal.alarm(7200 if args.work_probes else 600)
        try:
            packet(args, pins)
            authority = cold(P / 'qualified_preparation.py', 'prolog_prepared_work_actual_authority', pins)
            for owner in authority.leases():
                lease = dependencies.enter_context((owner / 'owner.lock').open('rb'))
                fcntl.flock(lease, fcntl.LOCK_SH | fcntl.LOCK_NB)
                held, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
                assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
            modules = authority.load(pins)
            report.update(reused_preparation_report_sha256=authority.REPORT_SHA,
                reused_runtime_sha256=modules['prep'].RUNTIME_SHA,
                preserved_preparation_profile_count=20, preserved_preparation_case_count=156,
                retained_contract_fixture_count=6, retained_ownership_fixture_count=11)
            if args.fixtures:
                fixture = cold(P / 'fixture_runner.py', 'prolog_prepared_original_work_fixtures', pins)
                sources = {(row['benchmark'], row['variant']): row['measured_source']
                    for row in modules['report']['profiles'].values()
                    if row['benchmark'] in ('binarytrees', 'pidigits', 'knucleotide')}
                assert len(sources) == 6
                report['work_fixture_receipt'] = fixture.run(output, pins, sha, authority.verify, save, sources)
                report['status'] = 'six-work-fixtures-passed-awaiting-thirty-probes'
            else:
                report['reused_work_fixture_receipt'] = closed_fixtures(args, pins, authority, dependencies)
                report['new_work_fixture_reexecutions'] = 0
                probes(output, modules, pins, common.fileno(), report)
        except BaseException as error:
            failure = error
            (output / 'error.log').write_text(traceback.format_exc())
        finally:
            signal.alarm(0)
            if modules is not None:
                modules['prep'].terminal(output, report, pins, failure)
            else:
                errors, actual = [], {}
                for path, expected in pins.items():
                    try:
                        actual[path] = sha(path)
                        assert actual[path] == expected
                    except BaseException as error:
                        errors.append(dict(path=path, error=str(error)))
                for path in output.rglob('*'):
                    if path.is_file():
                        try:
                            actual[str(path)] = sha(path)
                        except BaseException as error:
                            errors.append(dict(path=str(path), error=str(error)))
                report.update(status='failed-before-original-service', error=str(failure), pins=pins,
                    available_pins=actual, terminal_artifact_errors=errors, terminal_evidence_complete=not errors)
                save(output / 'report.json', report)
                save(output / 'terminal-pin.json', dict(status=report['status'], report=str(output / 'report.json'),
                    report_sha256=sha(output / 'report.json'), partial_evidence_pins=actual,
                    inherited_common_mutex=True, chart_admission=False, full_processes_authorized=0))
            if failure is not None:
                raise failure


if __name__ == '__main__':
    main()
