#!/usr/bin/env python3
"""Queue only C3 grammar fixtures, without source counting or chart admission.

Queueing copies this proposal and two already-frozen language-neutral modules.
Every import, fixture, dependency hash and pin reconciliation holds the common
timing mutex. A failed job remains preserved and is never automatically retried.
"""
import argparse
import fcntl
import json
import os
import shutil
import subprocess
import sys
import traceback
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
PROPOSAL_FILES = ('c3_mask.py', 'foreign_c3.py', 'test_foreign_c3.py', 'REVIEW.md')
CORE_FILES = ('foreign_core.py', 'canonical.py')
EXPECTED_PYGMENTS = '2.19.2'
EXPECTED_FIXTURE_METHODS = 19
EXPECTED_FIXTURE_IDS = tuple('test_foreign_c3.C3Fixtures.' + name for name in (
    'test_module_import_and_separate_packed_statements',
    'test_c3_function_formals_are_structural',
    'test_extern_abi_signature_and_attribute_are_one_header',
    'test_single_external_variable_has_proved_typed_owner',
    'test_function_pointer_and_generic_aliases',
    'test_struct_pointer_array_slice_fields_and_independent_declarators',
    'test_nested_struct_has_its_own_real_close',
    'test_initialized_uninitialized_and_chained_bindings',
    'test_for_foreach_headers_do_not_become_body_assignments',
    'test_inline_control_branch_and_body_count',
    'test_defer_call_and_defer_block_owners',
    'test_pure_arrow_one_definition_and_mutating_arrow_is_gated',
    'test_named_call_arguments_and_data_assignments_are_structural',
    'test_initializer_and_index_owners_cannot_hide_executable_effects',
    'test_scalar_switch_arm_headers_preserve_each_real_body_statement',
    'test_nested_comments_literals_raw_ticks_and_exact_offsets',
    'test_contracts_and_macro_compile_time_forms_are_not_silent_comments',
    'test_unterminated_and_unbalanced_input_remains_gated',
    'test_optional_unwrapping_unknown_attribute_and_switch_are_gated',
))
TERMINAL_PRIOR_STATUSES = {'failed-fixtures'}


def now():
    return datetime.now(timezone.utc).isoformat()


def write_json(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def state(job, status, **fields):
    value = dict(status=status, updated_at=now(), pid=os.getpid(),
                 pgid=os.getpgrp(), automatic_retry=False,
                 source_counting_started=False, chart_admitted=False, **fields)
    temp = job / 'state.json.tmp'
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(job / 'state.json')


def locked_pins(job, dependency_root, prior_receipt):
    # Called only after acquiring LOCK, before importing proposal/dependencies.
    import hashlib
    snapshot = job / 'snapshot'
    expected = set((*PROPOSAL_FILES, *CORE_FILES))
    if {path.name for path in snapshot.iterdir()} != expected:
        raise RuntimeError('Frozen snapshot inventory differs from reviewed files.')
    paths = [snapshot / name for name in sorted(expected)]
    paths += [job / 'worker.py', Path(sys.executable).resolve(),
              prior_receipt, job / 'prior-fixture-receipt.json',
              prior_receipt.parent / 'fixtures.log', prior_receipt.parent / 'fixture-receipt-pin.json']
    package = dependency_root / 'pygments'
    if not package.is_dir():
        raise RuntimeError('Reviewed Pygments dependency package is absent.')
    paths += sorted(package.rglob('*.py'))
    return {str(path.resolve()): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in paths}


def worker(job):
    config = json.loads((job / 'launch.json').read_text())
    # Job ownership is independent of the benchmark mutex. Duplicate workers
    # cannot execute or overwrite this preserved receipt.
    with (job / 'worker-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'fixture-receipt.json').exists():
            raise RuntimeError('This preserved fixture job already has a receipt.')
        state(job, 'waiting-for-common-timing-mutex', queued_at=config['queued_at'])
        print('WAIT LOCK: C3 proposal fixtures and dependency pins', flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            started_at = now()
            receipt = dict(status='failed-stop', started_at=started_at,
                common_timing_mutex=str(LOCK), exclusive_mutex_held=True,
                pid=os.getpid(), pgid=os.getpgrp(),
                python_executable=sys.executable, python_version=sys.version,
                frozen_sources=config['frozen_sources'],
                benchmark_processes_started=0, subprocesses_started_by_fixtures=0,
                source_counting_started=False, chart_admitted=False,
                automatic_retry=False, compiler_version='0.8.5',
                compiler_source_commit='640091d3303a393f9651d644063b837fc19d05f1')
            terminal_status = 'failed-stop'
            try:
                descriptor = os.fstat(lock.fileno())
                named = LOCK.stat()
                if (descriptor.st_dev, descriptor.st_ino) != (named.st_dev, named.st_ino):
                    raise RuntimeError('Mutex descriptor/path identity differs.')
                receipt['mutex_identity'] = dict(device=descriptor.st_dev,
                    inode=descriptor.st_ino, descriptor=lock.fileno())
                if sys.version_info[:2] != (3, 14) or Path(sys.executable).resolve() != Path(config['python_executable']).resolve():
                    raise RuntimeError('Worker must use the reviewed actual Python 3.14 executable.')
                dependency_root = Path(config['dependency_root']).resolve()
                prior_receipt = Path(config['prior_fixture_receipt']).resolve()
                prior = json.loads(prior_receipt.read_text())
                if prior['status'] not in TERMINAL_PRIOR_STATUSES or prior['exclusive_mutex_held'] is not True or prior['source_counting_started'] is not False or prior['chart_admitted'] is not False:
                    raise RuntimeError('V3 prerequisite is not the actual unscored failed fixture receipt.')
                if (prior['tests_run'], prior['failures'], prior['errors'], prior['skipped'], prior['expected_failures'], prior['unexpected_successes']) != (19, 1, 0, 0, 0, 0):
                    raise RuntimeError('V3 prerequisite differs from the preserved nineteen-test/one-failure outcome.')
                if prior['exact_test_execution'] is not True or prior['pin_reconciliation_passed'] is not True or sorted(prior['discovered_ids']) != sorted(EXPECTED_FIXTURE_IDS) or sorted(prior['started_ids']) != sorted(EXPECTED_FIXTURE_IDS):
                    raise RuntimeError('V3 actual discovered/started fixture inventory is not the exact reviewed nineteen.')
                before = locked_pins(job, dependency_root, prior_receipt)
                if before[str(prior_receipt)] != before[str((job / 'prior-fixture-receipt.json').resolve())]:
                    raise RuntimeError('Preserved V3 terminal receipt differs from queued snapshot.')
                prior_log = prior_receipt.parent / 'fixtures.log'
                prior_pin_path = prior_receipt.parent / 'fixture-receipt-pin.json'
                prior_pin = json.loads(prior_pin_path.read_text())
                if (Path(prior['raw_fixture_log']['path']).resolve() != prior_log or
                    prior['raw_fixture_log']['sha256'] != before[str(prior_log)] or
                    prior_pin['common_mutex_held'] is not True or Path(prior_pin['receipt']).resolve() != prior_receipt or
                    prior_pin['receipt_sha256'] != before[str(prior_receipt)] or
                    prior_pin['terminal_worker_status'] != 'failed-fixtures' or prior_pin['saved_receipt_status'] != 'failed-fixtures' or
                    prior_pin['raw_fixture_log'] != prior['raw_fixture_log']):
                    raise RuntimeError('V3 failed receipt/raw-log pin is not bound to the actual prerequisite files.')
                receipt['pins_before'] = before
                receipt['prior_fixture_receipt'] = str(prior_receipt)
                receipt['prior_fixture_status'] = prior['status']
                import importlib
                import unittest
                import hashlib
                sys.path.insert(0, str(dependency_root))
                sys.path.insert(0, str(job / 'snapshot'))
                pygments = importlib.import_module('pygments')
                if pygments.__version__ != EXPECTED_PYGMENTS:
                    raise RuntimeError('Reviewed Pygments version differs.')
                if Path(pygments.__file__).resolve().parent != dependency_root / 'pygments':
                    raise RuntimeError('Pygments import escaped the reviewed dependency root.')
                receipt['pygments_version'] = pygments.__version__
                receipt['pygments_module'] = str(Path(pygments.__file__).resolve())
                module = importlib.import_module('test_foreign_c3')
                for name in ('test_foreign_c3', 'foreign_c3', 'c3_mask', 'foreign_core', 'canonical'):
                    loaded = sys.modules[name]
                    if Path(loaded.__file__).resolve() != job / 'snapshot' / (name + '.py'):
                        raise RuntimeError('Proposal import escaped immutable snapshot: ' + name)
                suite = unittest.defaultTestLoader.loadTestsFromModule(module)
                if suite.countTestCases() != EXPECTED_FIXTURE_METHODS:
                    raise RuntimeError('Reviewed fixture method inventory must be exactly nineteen.')
                def ids(tests):
                    for test in tests:
                        if isinstance(test, unittest.TestSuite): yield from ids(test)
                        else: yield test.id()
                discovered = list(ids(suite))
                if len(discovered) != EXPECTED_FIXTURE_METHODS or sorted(discovered) != sorted(EXPECTED_FIXTURE_IDS):
                    raise RuntimeError('Discovered fixture identities differ from the exact reviewed nineteen.')
                class StrictResult(unittest.TextTestResult):
                    def __init__(self, *args, **kwargs):
                        super().__init__(*args, **kwargs)
                        self.started_ids = []
                    def startTest(self, test):
                        self.started_ids.append(test.id())
                        super().startTest(test)
                state(job, 'fixtures-running', started_at=started_at,
                      expected_tests=EXPECTED_FIXTURE_METHODS)
                print('Acquired common timing mutex; testing frozen C3 proposal.', flush=True)
                with (job / 'fixtures.log').open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=StrictResult).run(suite)
                exact_execution = (result.testsRun == EXPECTED_FIXTURE_METHODS and
                    len(result.started_ids) == EXPECTED_FIXTURE_METHODS and
                    sorted(result.started_ids) == sorted(EXPECTED_FIXTURE_IDS))
                complete = (result.wasSuccessful() and exact_execution and not result.skipped and
                            not result.expectedFailures and not result.unexpectedSuccesses)
                receipt.update(tests_run=result.testsRun, failures=len(result.failures),
                    errors=len(result.errors), skipped=len(result.skipped),
                    expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                    discovered_ids=discovered, started_ids=result.started_ids, expected_ids=list(EXPECTED_FIXTURE_IDS),
                    exact_test_execution=exact_execution, suite='test_foreign_c3',
                    raw_fixture_log=dict(path=str(job / 'fixtures.log'),
                        sha256=hashlib.sha256((job / 'fixtures.log').read_bytes()).hexdigest()),
                    status='passed-awaiting-source-review' if complete else 'failed-fixtures')
                after = locked_pins(job, dependency_root, prior_receipt)
                receipt['pins_after'] = after
                receipt['pin_reconciliation_passed'] = before == after
                if before != after:
                    raise RuntimeError('Frozen executed dependency bytes changed during fixtures.')
                # No authored benchmark source is imported, read or normalized.
                # This receipt qualifies only grammar fixtures for root review.
                receipt['finished_at'] = now()
                write_json(job / 'fixture-receipt.json', receipt)
                state(job, receipt['status'], tests_run=result.testsRun,
                      failures=len(result.failures), errors=len(result.errors))
                print(json.dumps(dict(status=receipt['status'], tests_run=result.testsRun,
                    failures=len(result.failures), errors=len(result.errors),
                    source_counting_started=False, chart_admitted=False)), flush=True)
                terminal_status = receipt['status']
                return 0 if complete else 1
            except BaseException as error:
                terminal_status = 'failed-stop'
                with (job / 'exception.log').open('x') as stream:
                    stream.write(traceback.format_exc())
                traceback.print_exc()
                receipt.update(status='failed-stop', error_type=type(error).__name__,
                               error=str(error), finished_at=now())
                if not (job / 'fixture-receipt.json').exists():
                    write_json(job / 'fixture-receipt.json', receipt)
                state(job, 'failed-stop', error_type=type(error).__name__, error=str(error))
                return 1
            finally:
                # Remains inside the actual held common flock on every return
                # and exception. The saved receipt is immutable even when a
                # later operation fails; the terminal status records that gap.
                import hashlib
                receipt_path = job / 'fixture-receipt.json'
                available = [path for path in (receipt_path, job / 'fixtures.log', job / 'exception.log') if path.is_file()]
                terminal_pins = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in available}
                saved_status = json.loads(receipt_path.read_text())['status'] if receipt_path.is_file() else None
                raw_log = job / 'fixtures.log'
                write_json(job / 'fixture-receipt-pin.json', dict(
                    common_mutex_held=True, terminal_worker_status=terminal_status,
                    saved_receipt_status=saved_status, terminal_artifact_pins=terminal_pins,
                    receipt=str(receipt_path) if receipt_path.is_file() else None,
                    receipt_sha256=terminal_pins.get(str(receipt_path)),
                    raw_fixture_log=dict(path=str(raw_log), sha256=terminal_pins[str(raw_log)]) if raw_log.is_file() else None,
                    automatic_retry=False, source_counting_started=False, chart_admitted=False))
                # Exclusive creation/hash/write failures propagate fatally;
                # they cannot become a successful qualification return.


def queue(job, core_snapshot, dependency_root, prior_receipt, direct=False):
    job, core_snapshot = job.resolve(), core_snapshot.resolve()
    dependency_root = dependency_root.resolve()
    prior_receipt = prior_receipt.resolve()
    if not prior_receipt.is_file() or json.loads(prior_receipt.read_text()).get('status') not in TERMINAL_PRIOR_STATUSES:
        raise SystemExit('V4 cannot queue before the preserved V3 failed terminal outcome exists.')
    job.mkdir(parents=True, exist_ok=True)
    with (job / 'queue.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if (job / 'launch.json').exists() or (job / 'snapshot').exists():
            raise SystemExit('Preserved C3 fixture job already queued; choose an additive revision.')
        sources = [HERE / name for name in PROPOSAL_FILES]
        sources += [core_snapshot / name for name in CORE_FILES]
        if any(not path.is_file() for path in sources):
            raise SystemExit('Reviewed snapshot source is absent.')
        snapshot = job / 'snapshot'
        snapshot.mkdir()
        for source in sources:
            target = snapshot / source.name
            shutil.copyfile(source, target)
            target.chmod(0o444)
        snapshot.chmod(0o555)
        runner = job / 'worker.py'
        shutil.copyfile(Path(__file__), runner)
        runner.chmod(0o444)
        shutil.copyfile(prior_receipt, job / 'prior-fixture-receipt.json')
        (job / 'prior-fixture-receipt.json').chmod(0o444)
        write_json(job / 'launch.json', dict(queued_at=now(), mutex=str(LOCK),
            frozen_sources=[str(path) for path in sources], snapshot_files=[path.name for path in sources],
            frozen_worker_source=str(Path(__file__).resolve()),
            dependency_root=str(dependency_root), expected_pygments_version=EXPECTED_PYGMENTS,
            prior_fixture_receipt=str(prior_receipt),
            controller_mode='same-pid-direct-exec' if direct else 'detached-unused-candidate',
            expected_fixture_methods=EXPECTED_FIXTURE_METHODS,
            python_executable=sys.executable, python_version=sys.version,
            automatic_retry=False, source_counting_started=False, chart_admitted=False))
        command = [sys.executable, '-B', '-u', str(runner), '--worker', str(job)]
        if direct:
            # Record before any process handoff. exec preserves this exact PID;
            # no detached child can outlive a failed post-Popen journal write.
            write_json(job / 'process.json', dict(pid=os.getpid(), pgid=os.getpgrp(),
                launched_at=now(), command=command, same_pid_direct_exec=True,
                automatic_retry=False, source_counting_started=False, chart_admitted=False))
            print(json.dumps(dict(status='frozen-awaiting-same-pid-worker-exec',
                pid=os.getpid(), job=str(job), mutex=str(LOCK))), flush=True)
            try:
                os.execv(sys.executable, command)
            except BaseException as error:
                write_json(job / 'direct-exec-failure.json', dict(status='failed-stop',
                    error=str(error), error_type=type(error).__name__, native_children_started=0,
                    source_counting_started=False, chart_admitted=False, automatic_retry=False))
                raise
        with (job / 'stdout.log').open('xb') as log:
            process = subprocess.Popen([sys.executable, '-B', '-u', str(runner), '--worker', str(job)],
                stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                start_new_session=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
        write_json(job / 'process.json', dict(pid=process.pid, pgid=process.pid,
            launched_at=now(), command=[sys.executable, '-B', '-u', str(runner), '--worker', str(job)],
            automatic_retry=False, source_counting_started=False, chart_admitted=False))
        print(json.dumps(dict(status='queued', pid=process.pid, job=str(job), mutex=str(LOCK))))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--direct', action='store_true',
                        help='Freeze then exec the immutable worker in this same PID; reviewed launch mode.')
    parser.add_argument('--job', type=Path, default=HERE / 'jobs/fixtures-v4')
    parser.add_argument('--core-snapshot', type=Path,
                        default=HERE.parent / 'jobs/boundary-audit-18/snapshot')
    parser.add_argument('--dependency-root', type=Path,
                        default=Path('/tmp/mech-clbg-compact-libs'))
    parser.add_argument('--prior-fixture-receipt', type=Path,
                        default=HERE.parent / 'c3-proposal-v3/jobs/fixtures-v3/fixture-receipt.json')
    args = parser.parse_args()
    if args.worker:
        if args.direct: parser.error('--direct cannot accompany --worker')
        return worker(args.worker.resolve())
    queue(args.job, args.core_snapshot, args.dependency_root, args.prior_fixture_receipt, args.direct)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
