"""Unexecuted additive MATLAB semantic qualification and full-study candidate.

All runtime inspection, hashing, oracle generation and native work hold the
common mutex. Metadata sealing is a separate reviewed step before execution.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import statistics
import subprocess
import threading
import time
import unittest

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[2]
OLD = CLBG / 'matlab-comparison/20261010'
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
GUARD = SUITE / 'uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
LOCK = Path('/tmp/mech-clbg-timing.lock')
GATES = {'sampler_exited_normally','authoritative_native_group_received','native_inherited_common_mutex',
    'native_exited_normally','no_execution_error','no_wrapper_integrity_error','no_resource_violation',
    'no_resource_monitor_errors','no_compiler_power_monitor_errors','no_monitor_finish_error',
    'no_execute_cleanup_error','no_remaining_execute_group','no_supervisor_cleanup',
    'no_supervisor_remaining_group','valid_environment','actual_full_workload','exact_command_and_group',
    'complete_output_reference_match','complete_output_bytes_and_hash','pins_unchanged'}
SAFE = GATES - {'native_exited_normally','no_execution_error','no_resource_violation',
    'complete_output_reference_match','complete_output_bytes_and_hash'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def new(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def guard_at(output):
    assert sha(GUARD) == GUARD_SHA
    guard = module('matlab_case_frozen_guard', GUARD)
    guard.OUT = output
    guard.REPORT = output / 'report.json'
    return guard, guard.load_harness()


def complete(attempt, row, harness, full):
    assert attempt['qualified'] is True and attempt['full_workload'] is full
    assert type(attempt['exit_status']) is type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert type(attempt['qualification_checks']) is dict and set(attempt['qualification_checks']) == GATES
    assert all(value is True for value in attempt['qualification_checks'].values())
    assert attempt['source_sha256'] == row['source_sha256']
    assert attempt['execution_fingerprint'] == harness.execution_fingerprint(row)
    assert attempt['command'] == [part.replace('{n}',str(attempt['workload_n'])) for part in row['command']]
    if row['language'] == 'matlab' and not full:
        proof = attempt['bounded_mutex_fd_evidence']
        assert proof['descriptor'] != 3 and proof['native_MATLAB_engine_observed'] is True
        assert proof['observations'] and proof['errors'] == []


def isolated(guard, attempt):
    checks = attempt.get('qualification_checks')
    return (type(checks) is dict and set(checks) == GATES
        and all(type(value) is bool for value in checks.values())
        and guard.benchmark_failure(attempt) and all(checks[key] is True for key in SAFE))


def bounded_descriptor_observer(harness, row, descriptor, evidence_dir):
    """Observe actual group FDs during qualification; never add a timing monitor."""
    execute, popen = harness.execute, harness.subprocess.Popen
    native_command = row['command']
    proof = dict(descriptor=descriptor, native_MATLAB_engine_observed=False, observations=[], errors=[])
    stopped = threading.Event()
    worker = None

    def observe(process):
        while not stopped.is_set() and process.poll() is None:
            try:
                command = ['/usr/sbin/lsof','-n','-P','-a','-g',str(process.pid),'-d',str(descriptor),'-Fpcfn']
                evidence_dir.mkdir(parents=True,exist_ok=True)
                prefix = evidence_dir / f'{len(proof["observations"])+len(proof["errors"])+1:04d}-lsof'
                check = popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    start_new_session=True,pass_fds=(descriptor,))
                stdout,stderr = b'',b''
                detail = dict(command=command,pid=check.pid,native_group=process.pid,descriptor=descriptor,
                    inherited_common_mutex=True,timeout_seconds=5,preparation_only=True)
                try:
                    stdout, stderr = check.communicate(timeout=5)
                except subprocess.TimeoutExpired as error:
                    stdout,stderr = error.output or b'',error.stderr or b''
                    detail.update(error=str(error),error_type=type(error).__name__)
                    raise
                finally:
                    try:
                        if check.poll() is None:
                            harness.kill_group(check.pid)
                            check.wait(timeout=5)
                        remaining = harness.group_members(check.pid)
                        detail.update(exit_status=check.returncode,remaining_group_members=remaining)
                        assert not remaining, 'Descriptor observer child group remained'
                    except BaseException as error:
                        detail.update(cleanup_error=str(error),cleanup_error_type=type(error).__name__)
                        raise
                    finally:
                        prefix.with_suffix('.stdout').write_bytes(stdout)
                        prefix.with_suffix('.stderr').write_bytes(stderr)
                        new(prefix.with_suffix('.json'),detail)
                text = stdout.decode(errors='replace')
                held = 'n' + str(LOCK.resolve()) in text or 'n' + str(LOCK) in text
                if check.returncode not in (0,1) or not held:
                    # Do not misclassify a group that exited during lsof.
                    if process.poll() is None:
                        proof['errors'].append(dict(command=command, exit_status=check.returncode,
                            stdout=text, stderr=stderr.decode(errors='replace')))
                        harness.kill_group(process.pid)
                    return
                proof['observations'].append(dict(at=time.time(), native_group=process.pid, lsof=text))
                records,record = [],None
                for line in text.splitlines():
                    if line.startswith('p'):
                        record = {'pid':line[1:],'names':[]}
                        records.append(record)
                    elif record is not None and line.startswith('c'):
                        record['command'] = line[1:]
                    elif record is not None and line.startswith('n'):
                        record['names'].append(line[1:])
                if any(record.get('command') == 'MATLAB' and any(name in (str(LOCK),str(LOCK.resolve()))
                        for name in record['names']) for record in records):
                    proof['native_MATLAB_engine_observed'] = True
            except BaseException as error:
                proof['errors'].append(dict(error=str(error), error_type=type(error).__name__))
                harness.kill_group(process.pid)
                return
            stopped.wait(.2)

    def observed_popen(*args, **kwargs):
        nonlocal worker
        process = popen(*args, **kwargs)
        argv = args[0] if args else kwargs.get('args')
        if list(argv) == native_command and kwargs.get('start_new_session'):
            worker = threading.Thread(target=observe, args=(process,), daemon=True)
            worker.start()
        return process

    def observed_execute(*args, **kwargs):
        try:
            result = execute(*args, **kwargs)
        finally:
            stopped.set()
            if worker:
                worker.join(timeout=10)
                if worker.is_alive():
                    proof['errors'].append(dict(error='Descriptor observer did not join'))
        result['bounded_mutex_fd_evidence'] = proof
        if proof['errors'] or not proof['native_MATLAB_engine_observed']:
            result.update(supervisor_error='Actual MATLAB common-mutex descriptor ownership unproved')
        return result
    harness.subprocess.Popen, harness.execute = observed_popen, observed_execute
    return lambda: (setattr(harness.subprocess, 'Popen', popen), setattr(harness, 'execute', execute))


def one(guard, harness, row, manifest, report, round_index, lock, *, data=None, expected=None, case=None):
    previous_work, previous_input, previous_validate = harness.WORK['regexredux'], harness.INPUTS['regexredux'], harness.validate
    duplicate = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD_CLOEXEC, 64)
    restore = lambda: None
    try:
        assert duplicate != 3 and os.fstat(duplicate).st_ino == os.fstat(lock.fileno()).st_ino
        if case is not None:
            directory = guard.OUT / 'cases' / row['id'] / case
            directory.mkdir(parents=True)
            (directory / 'stdin').write_bytes(data)
            (directory / 'expected.stdout').write_bytes(expected)
            case_pins = {str(path): sha(path) for path in (directory/'stdin',directory/'expected.stdout')}
            report.setdefault('case_pins',{}).update(case_pins)
            harness.INPUTS['regexredux'] = (directory / 'stdin', previous_input[1])
            harness.WORK['regexredux'] = (1000, previous_work[1])
            manifest = dict(manifest, size=1000, canonical_full_workload=False, pins=dict(manifest['pins'],**case_pins),
                full_output_reference=dict(bytes=len(expected), sha256=hashlib.sha256(expected).hexdigest()))
            def validate(benchmark, full, stream):
                actual = stream.read()
                (directory / 'actual.stdout').write_bytes(actual)
                assert benchmark == 'regexredux' and full is False and actual == expected
                return dict(reference_match=True, bytes=len(actual), sha256=hashlib.sha256(actual).hexdigest())
            harness.validate = validate
            if row['language'] == 'matlab':
                restore = bounded_descriptor_observer(harness, row, duplicate,directory/'descriptor-observer')
        try:
            attempt = guard.one_attempt(harness, row, manifest, report, round_index, duplicate)
        except BaseException as error:
            index = len(report['attempts']) + 1
            raw = guard.OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
            saved = raw.with_suffix('.supervisor.json')
            attempt = read(saved if saved.exists() else raw) if saved.exists() or raw.exists() else {}
            attempt.update(qualified=False, attempt=index, id=row['id'], wrapper_supervisor_failure=True,
                wrapper_error=str(error) or type(error).__name__, wrapper_error_type=type(error).__name__)
            guard.save(raw.with_suffix('.wrapper.json'), attempt)
            report['attempts'].append(attempt)
            report['active'] = None
            guard.save(guard.REPORT, report)
            raise
        if case is not None:
            attempt.update(case=case, preparation_only=True, full_performance_run=False, chart_admission=False)
        else:
            attempt.update(full_performance_run=True, chart_admission=False)
        report['attempts'].append(attempt)
        report['active'] = None
        guard.save(guard.REPORT, report)
        return attempt
    finally:
        restore()
        os.close(duplicate)
        harness.WORK['regexredux'], harness.INPUTS['regexredux'], harness.validate = previous_work, previous_input, previous_validate


def seal():
    guard, harness = guard_at(HERE / 'qualification-v1')
    fixture_module = module('matlab_case_controller_fixtures',HERE/'test_controller.py')
    fixture_suite = unittest.defaultTestLoader.loadTestsFromModule(fixture_module)
    def identities(suite):
        return [identity for item in suite for identity in (identities(item) if isinstance(item,unittest.TestSuite) else [item.id()])]
    expected_tests = ['matlab_case_controller_fixtures.CandidateMocks.' + name for name in (
        'test_attempt_failure_preserves_raw_and_restores_overrides',
        'test_observer_restores_exact_original_functions',
        'test_owned_observer_timeout_cleans_only_known_mock_groups')]
    assert sorted(identities(fixture_suite)) == sorted(expected_tests)
    fixture_log = io.StringIO()
    class StartedTests(unittest.TextTestResult):
        def __init__(self,*args,**kwargs):
            super().__init__(*args,**kwargs)
            self.started_ids = []
        def startTest(self,test):
            self.started_ids.append(test.id())
            super().startTest(test)
    result = unittest.TextTestRunner(stream=fixture_log,verbosity=2,resultclass=StartedTests).run(fixture_suite)
    fixture_dir = HERE/'metadata-seal-v1'
    fixture_dir.mkdir()
    (fixture_dir/'fixtures.log').write_text(fixture_log.getvalue())
    fixture_pins = {str(path):sha(path) for path in (Path(__file__),HERE/'test_controller.py',fixture_dir/'fixtures.log')}
    new(fixture_dir/'fixture-receipt.json',dict(status='passed' if result.wasSuccessful() else 'failed-fixtures',
        tests_run=result.testsRun,expected_tests=expected_tests,actual_started_ids=result.started_ids,
        failures=len(result.failures),errors=len(result.errors),expected_failures=len(result.expectedFailures),
        unexpected_successes=len(result.unexpectedSuccesses),
        skipped=len(result.skipped),actual_native_subjects=0,actual_signals_sent=0,
        common_timing_mutex=str(LOCK),exclusive_mutex_held=True,pins=fixture_pins))
    assert result.wasSuccessful() and result.testsRun == 3 and not result.skipped
    assert result.started_ids == sorted(expected_tests) and not result.expectedFailures and not result.unexpectedSuccesses
    builds_path = OLD / 'results/builds.json'
    builds = read(builds_path)
    original = [row for row in builds['programs'] if row['benchmark'] == 'regexredux']
    assert len(original) == 2 and {row['profile'] for row in original} == {'compact','optimized'}
    pins = {str(path.resolve()): sha(path) for path in [Path(__file__), HERE/'proposal-draft.json',
        HERE/'QUALIFICATION_PLAN.md', HERE/'runtime_identity.m', HERE/'test_controller.py',
        fixture_dir/'fixtures.log',fixture_dir/'fixture-receipt.json',builds_path, GUARD, SUITE/'run.py', SUITE/'prepare.py']}
    runtime = {item['path']: item['sha256'] for item in builds['runtimes'].values() if 'path' in item}
    guard.verify(runtime)
    pins.update(runtime)
    profiles = []
    for old in original:
        assert sha(old['source']) == old['source_sha256']
        source = HERE / 'sources' / Path(old['source']).name
        expected = Path(old['source']).read_text().replace('regexpi(', 'regexp(')
        uncomment = lambda text: '\n'.join(line for line in text.splitlines() if not line.lstrip().startswith('%'))
        assert uncomment(source.read_text()) == uncomment(expected)
        pins[str(source)] = sha(source)
        pins[old['source']] = old['source_sha256']
        row = copy.deepcopy(old)
        row.pop('loc_audit',None)
        row.update(id='matlab-case-' + old['profile'], key='regexredux:matlab-case-' + old['profile'],
            variant=old['profile'], source=str(source), source_sha256=sha(source), cwd=str(HERE), loc=None,
            producer_loc=None, binary='/bin/sh', binary_sha256=sha('/bin/sh'), maximum_workers=8,
            runtime_environment=dict(old['runtime_environment'], LC_ALL='C', MATLABPATH=''), support=dict(runtime))
        row['command'][row['command'].index('-sd')+1] = str(source.parent)
        profiles.append(row)
    pins['/bin/sh'] = sha('/bin/sh')
    audit_path = SUITE/'source-audit-r.json'
    audit = read(audit_path)
    assert audit['status'] == 'reviewed'
    control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == 'regexredux'))
    control.update(id='mech-aot-control', key='regexredux:matlab-case:mech-control',
        producer_loc=control.get('loc'), loc=None, maximum_workers=8)
    pins.update(audit['pins'])
    pins[str(audit_path)] = sha(audit_path)
    profiles.append(control)
    references_path = CLBG/'v-report-only/20261012/references.py'
    references = module('matlab_case_byte_oracle', references_path)
    oracle_receipt = read(references_path.parent/'oracle-fixtures.json')
    assert oracle_receipt['status'] == 'passed' and oracle_receipt['failures'] == oracle_receipt['errors'] == 0
    assert oracle_receipt['tests_run'] == 14 and oracle_receipt['shared_timing_lock'] == str(LOCK)
    guard.verify(oracle_receipt['pins'])
    pins.update(oracle_receipt['pins'])
    for path in [references_path,references_path.parent/'test_references.py',references_path.parent/'oracle-fixtures.json']:
        pins[str(path)] = sha(path)
    cases = {'empty-input': b'', 'canonical-small': harness.INPUTS['regexredux'][0].read_bytes()}
    for path in sorted((HERE/'fixtures').glob('*.fasta')):
        cases[path.stem] = path.read_bytes()
        pins[str(path)] = sha(path)
    assert set(cases) == {'empty-input','canonical-small','first-pattern-case','all-nine-case','ordered-substitutions'}
    expected = {case: references.regexredux(data).hex() for case,data in cases.items()}
    assert bytes.fromhex(expected['first-pattern-case']).startswith(b'agggtaaa|tttaccct 1\n')
    reference = CLBG/'regexredux/inputs/output-full.txt'
    for path in (*harness.INPUTS['regexredux'], reference):
        pins[str(path)] = sha(path)
    guard.verify(pins)
    proposal = dict(status='sealed-awaiting-root-native-review', profiles=profiles, original_profiles=original,
        cases={case:data.hex() for case,data in cases.items()}, expected=expected, pins=pins,
        full_output_reference=dict(bytes=reference.stat().st_size, sha256=sha(reference)), size=harness.WORK['regexredux'][1],
        samples=3, canonical_full_workload=True, shared_timing_lock=str(LOCK), maximum_rss_mib=10240,
        timeout_seconds=14400, maximum_compute_workers=8, required_power_profile=harness.REQUIRED,
        chart_admission=False, no_automatic_retry=True, source_LOC=None, source_work_review='Nine case-sensitive counts; five original ordered substitutions; exact new application copies')
    new(HERE/'proposal-sealed-v1.json', proposal)
    print(json.dumps(dict(proposal=str(HERE/'proposal-sealed-v1.json'), sha256=sha(HERE/'proposal-sealed-v1.json'))),flush=True)


def qualify(digest, lock):
    proposal_path = HERE/'proposal-sealed-v1.json'
    assert sha(proposal_path) == digest
    proposal = read(proposal_path)
    guard, harness = guard_at(HERE/'qualification-v1')
    assert not guard.REPORT.exists(), 'No implicit restart'
    guard.verify(proposal['pins'])
    harness.install_interrupt_handlers()
    harness.compact.wait_for_environment(harness.REQUIRED)
    report = dict(status='running', proposal=str(proposal_path), proposal_sha256=digest,
        profiles=proposal['profiles'], attempts=[], active=None, full_performance_runs=0, chart_admission=False,
        no_automatic_retry=True, case_pins={}, started_at=guard.now())
    guard.save(guard.REPORT,report)
    try:
        diagnostic = copy.deepcopy(proposal['profiles'][0])
        diagnostic.update(id='matlab-runtime-identity',key='regexredux:matlab-runtime-identity',
            source=str(HERE/'runtime_identity.m'),source_sha256=sha(HERE/'runtime_identity.m'))
        diagnostic['command'][diagnostic['command'].index('-sd')+1] = str(HERE)
        diagnostic['command'][-1] = 'runtime_identity()'
        attempt = one(guard,harness,diagnostic,proposal,report,0,lock,data=b'',
            expected=b'MATLAB R2026b native arm64 <=8 compute verified\n',case='runtime-identity')
        complete(attempt,diagnostic,harness,False)
        for row in proposal['profiles']:
            for case,payload in proposal['cases'].items():
                attempt = one(guard,harness,row,proposal,report,0,lock,data=bytes.fromhex(payload),
                    expected=bytes.fromhex(proposal['expected'][case]),case=case)
                complete(attempt,row,harness,False)
        expected = bytes.fromhex(proposal['expected']['first-pattern-case'])
        assert expected.startswith(b'agggtaaa|tttaccct 1\n')
        expected = expected.replace(b'agggtaaa|tttaccct 1\n',b'agggtaaa|tttaccct 2\n',1)
        for old in proposal['original_profiles']:
            row = copy.deepcopy(old)
            row.update(id='old-witness-' + old['profile'], key='regexredux:old-witness-' + old['profile'], cwd=str(HERE))
            attempt = one(guard,harness,row,proposal,report,0,lock,
                data=bytes.fromhex(proposal['cases']['first-pattern-case']),expected=expected,case='first-pattern-case')
            complete(attempt,row,harness,False)
        assert len(report['attempts']) == 18
        pins = dict(proposal['pins'],**report['case_pins'])
        pins[str(proposal_path)] = digest
        for path in guard.OUT.rglob('*'):
            if not path.is_file() or path == guard.REPORT:
                continue
            pins[str(path)] = sha(path)
        guard.verify(pins)
        report.update(status='semantic-output-runtime-ownership-passed-awaiting-full-review-and-LOC',
            pins=pins,pins_unchanged_at_end=True,finished_at=guard.now())
        guard.save(guard.REPORT,report)
        print(json.dumps(dict(report=str(guard.REPORT),sha256=sha(guard.REPORT))),flush=True)
    except BaseException as error:
        report.update(status='stopped-unqualified',error=str(error) or type(error).__name__,
            error_type=type(error).__name__,active=None,finished_at=guard.now())
        guard.save(guard.REPORT,report)
        raise


def seal_full(digest):
    path = HERE/'qualification-v1/report.json'
    assert sha(path) == digest
    qualified = read(path)
    assert qualified['status'] == 'semantic-output-runtime-ownership-passed-awaiting-full-review-and-LOC'
    assert qualified['active'] is None and qualified['full_performance_runs'] == 0
    assert qualified['pins_unchanged_at_end'] is True and len(qualified['attempts']) == 18
    proposal = read(qualified['proposal'])
    assert sha(qualified['proposal']) == qualified['proposal_sha256']
    guard,harness = guard_at(HERE/'studies/full-v1/regexredux')
    guard.verify(qualified['pins'])
    expected = {(row['id'],case) for row in proposal['profiles'] for case in proposal['cases']}
    observed = {(attempt['id'],attempt['case']) for attempt in qualified['attempts'] if attempt['id'] in {row['id'] for row in proposal['profiles']}}
    assert observed == expected and len(expected) == 15
    for row in proposal['profiles']:
        for attempt in qualified['attempts']:
            if attempt['id'] == row['id']:
                complete(attempt,row,harness,False)
    diagnostic = copy.deepcopy(proposal['profiles'][0])
    diagnostic.update(id='matlab-runtime-identity',key='regexredux:matlab-runtime-identity',
        source=str(HERE/'runtime_identity.m'),source_sha256=sha(HERE/'runtime_identity.m'))
    diagnostic['command'][diagnostic['command'].index('-sd')+1] = str(HERE)
    diagnostic['command'][-1] = 'runtime_identity()'
    originals = []
    for old in proposal['original_profiles']:
        row = copy.deepcopy(old)
        row.update(id='old-witness-' + old['profile'],key='regexredux:old-witness-' + old['profile'],cwd=str(HERE))
        originals.append(row)
    for row in [diagnostic,*originals]:
        attempts = [attempt for attempt in qualified['attempts'] if attempt['id'] == row['id']]
        assert len(attempts) == 1
        complete(attempts[0],row,harness,False)
    assert len({attempt['attempt'] for attempt in qualified['attempts']}) == 18
    manifest = copy.deepcopy(proposal)
    manifest.pop('cases'); manifest.pop('expected'); manifest.pop('original_profiles')
    ids = [row['id'] for row in proposal['profiles']]
    manifest.update(status='prepared-awaiting-root-review',benchmark='regexredux',qualified_semantics=str(path),
        qualified_semantics_sha256=digest,orders=[dict(round=i+1,profiles=ids[i:]+ids[:i]) for i in range(3)],
        pins=dict(qualified['pins'],**{str(path):digest}),full_processes=9)
    for row in manifest['profiles']:
        if row['language'] == 'matlab':
            row['required_work_review'] = dict(status='passed',source_sha256=row['source_sha256'],
                qualification=str(path),method='Exact source copy review plus full bounded byte outputs and explicit native MATLAB mutex/runtime evidence')
    new(guard.OUT/'manifest.json',manifest)
    index = dict(status='sealed-awaiting-root-review',benchmarks={'regexredux':dict(manifest=str(guard.OUT/'manifest.json'),sha256=sha(guard.OUT/'manifest.json'))},samples=3,full_processes=9,chart_admission=False)
    new(HERE/'studies/full-v1/index.json',index)
    print(json.dumps(dict(index=str(HERE/'studies/full-v1/index.json'),sha256=sha(HERE/'studies/full-v1/index.json'),benchmarks=index['benchmarks'])),flush=True)


def run_full(digest,lock):
    guard,harness = guard_at(HERE/'studies/full-v1/regexredux')
    path = guard.OUT/'manifest.json'
    assert sha(path) == digest
    manifest = read(path)
    assert manifest['status'] == 'prepared-awaiting-root-review' and manifest['size'] == harness.WORK['regexredux'][1]
    assert manifest['samples'] == 3 and manifest['canonical_full_workload'] is True
    assert not guard.REPORT.exists()
    guard.verify(manifest['pins'])
    harness.install_interrupt_handlers()
    rows = {row['id']:dict(copy.deepcopy(row),samples_seconds=[],sample_validation=[],attempts=[],measurement_status='pending') for row in manifest['profiles']}
    report = dict(status='running',manifest=str(path),manifest_sha256=digest,profiles=rows,
        orders=manifest['orders'],attempts=[],active=None,skipped_failed_profile_slots=[],
        started_at=guard.now(),chart_admission=False,no_automatic_retry=True)
    guard.save(guard.REPORT,report)
    try:
        for order in manifest['orders']:
            for identifier in order['profiles']:
                row = rows[identifier]
                if row.get('full_failure'):
                    report['skipped_failed_profile_slots'].append(dict(round=order['round'],id=identifier,
                        reason='Preserved earlier real source failure; no automatic retry'))
                    guard.save(guard.REPORT,report)
                    continue
                harness.compact.wait_for_environment(harness.REQUIRED)
                attempt = one(guard,harness,row,manifest,report,order['round'],lock)
                row['attempts'].append(attempt['attempt'])
                if attempt['qualified'] is True:
                    complete(attempt,row,harness,True)
                    row['samples_seconds'].append(attempt['seconds'])
                    row['sample_validation'].append(attempt['validation'])
                    row['median_seconds'] = statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds']) > 1:
                        row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                    row['measurement_status'] = 'measured' if len(row['samples_seconds']) == 3 else 'partial'
                else:
                    assert isolated(guard,attempt), 'Integrity/environment/ownership failure stops'
                    row.update(full_failure=copy.deepcopy(attempt),measurement_status='failed-full-source-unscored')
                    row.pop('median_seconds',None); row.pop('sample_stdev_seconds',None)
                guard.save(guard.REPORT,report)
        assert len(report['attempts']) + len(report['skipped_failed_profile_slots']) == 9
        assert all(len(row['samples_seconds']) == 3 or row.get('full_failure') for row in rows.values())
        guard.verify(manifest['pins'])
        report.update(status='complete-with-source-failures' if any(row.get('full_failure') for row in rows.values()) else 'complete',
            active=None,pins_unchanged_at_end=True,finished_at=guard.now())
        guard.save(guard.REPORT,report)
    except BaseException as error:
        report.update(status='stopped-unqualified',active=None,error=str(error) or type(error).__name__,
            error_type=type(error).__name__,finished_at=guard.now())
        guard.save(guard.REPORT,report)
        raise


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal',action='store_true')
    mode.add_argument('--qualify',action='store_true')
    mode.add_argument('--seal-full',action='store_true')
    mode.add_argument('--run-full',action='store_true')
    parser.add_argument('--reviewed-sha256')
    args = parser.parse_args()
    for sig in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP):
        signal.signal(sig,lambda signum,frame: (_ for _ in ()).throw(KeyboardInterrupt('MATLAB correction supervisor interrupted')))
    with (HERE/'owner.lock').open('a+') as owner:
        fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
        print('WAIT common mutex: MATLAB correction reviewed stage',flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            if args.seal:
                seal()
            else:
                assert args.reviewed_sha256 and len(args.reviewed_sha256) == 64
                if args.qualify: qualify(args.reviewed_sha256,lock)
                elif args.seal_full: seal_full(args.reviewed_sha256)
                else: run_full(args.reviewed_sha256,lock)


if __name__ == '__main__':
    main()
