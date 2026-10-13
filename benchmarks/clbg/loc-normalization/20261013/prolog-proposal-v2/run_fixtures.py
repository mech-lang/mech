"""Source-only owned runner for the unchanged thirty-one reader fixtures.

Root must supply an actual bootstrap report, terminal pin and reviewed scanner
runtime descriptor. No runtime download, qualification retry or benchmark runs.
All imports, hashing and scanner processes hold the common timing mutex.
"""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback
from types import SimpleNamespace

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[2]
LOCK = Path('/tmp/mech-clbg-timing.lock')
BOOTSTRAP = CLBG / 'prolog-comparison/20261013/revisions/runtime-v2/bootstrap_runtime_v2.py'
EXPECTED = tuple('test_prolog.ReaderOwners.' + name for name in (
    'test_each_fact_keeps_clause_dispatch',
    'test_formal_and_data_commas_are_structural',
    'test_packed_unification_arithmetic_cut_are_independent',
    'test_executable_packing_does_not_change_units',
    'test_decimal_and_exponent_dots_are_not_terminators',
    'test_literal_and_comment_punctuation_is_not_executable',
    'test_if_then_else_has_condition_arm_and_written_close_owners',
    'test_hidden_condition_binding_and_second_test_are_retained',
    'test_nested_disjunctive_condition_and_right_associated_if_chain',
    'test_nested_negation_and_meta_condition_headers_stay_independent',
    'test_vendor_meta_requires_selected_import_and_exact_target',
    'test_disjunction_does_not_turn_arms_into_argument_data',
    'test_catch_written_goal_and_recovery_bodies_are_exposed',
    'test_forwarded_meta_formal_is_a_reference_without_invented_body',
    'test_direct_variable_goal_keeps_its_written_meta_call',
    'test_literal_data_wrapping_is_preserved',
    'test_ordinary_call_argument_wrapping_does_not_expand',
    'test_utf8_character_positions_are_reconciled_to_original_bytes',
    'test_input_initialization_is_read_as_data_and_never_invoked',
    'test_unknown_call_with_hidden_meta_body_is_gated',
    'test_local_constructed_meta_target_is_gated',
    'test_binding_a_forwarded_meta_formal_is_gated',
    'test_helper_import_requires_its_actual_source_path',
    'test_unknown_unused_helper_import_is_gated',
    'test_same_path_context_requires_exact_source_ast_and_reader_settings',
    'test_operator_directive_is_not_applied',
    'test_supported_reader_syntax_is_not_unsupported_grammar_admission',
    'test_missing_full_stop_is_a_reader_error',
    'test_literal_end_of_file_cannot_hide_unread_source',
    'test_perl_language_does_not_select_prolog_for_pl_extension',
    'test_mismatched_original_bytes_and_changed_operator_inventory_are_gated',
))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)
        stream.write('\n')


def read(path, digest, pins):
    path = Path(path).resolve(strict=True)
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, 'Changed pinned input: ' + str(path))
    require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting source pin')
    pins[str(path)] = digest
    return raw


def fresh(path, name, pins):
    require(name not in sys.modules, 'Cached authored fixture module: ' + name)
    path = Path(path).resolve(strict=True)
    raw = read(path, pins[str(path)], pins)
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    require(sys.modules[name] is module, 'Fixture module identity changed')
    read(path, pins[str(path)], pins)
    return module


def run(args):
    # A new durable target owns this job's separate lease. Runtime runner
    # imports and their package snapshot also wait for the common lock.
    print('WAIT common mutex: thirty-one Prolog original-reader fixtures', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        actual, named = os.fstat(lock.fileno()), LOCK.stat()
        require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino), 'Different common mutex')
        output = Path(args.output).resolve()
        require(output.is_relative_to(HERE / 'jobs'), 'Fixture evidence must remain in this packet')
        output.mkdir(parents=True, exist_ok=False)
        pins, scanner_attempts, projections, started = {}, [], [], []
        discovered, live_result = [], None
        report = dict(status='failed-stop', expected_ids=list(EXPECTED),
            shared_timing_mutex_held=True, mutex_device=actual.st_dev, mutex_inode=actual.st_ino,
            benchmark_processes_started=0, source_counting_started=False,
            chart_admission=False, automatic_retry=False)
        error = None
        try:
            import subprocess
            import shutil
            import importlib
            require(not any(name == 'unittest' or name.startswith('unittest.')
                for name in sys.modules), 'Unittest must load from a fresh pinned package snapshot')
            sys.dont_write_bytecode = True
            review = json.loads(read(args.runtime_review, args.runtime_review_sha256, pins))
            require(review['status'] == 'reviewed-actual-swi-scanner-runtime' and
                review['review_identity'] and review['scanner_execution_authorized'] is True and
                review['benchmark_execution_authorized'] is False and review['unresolved'] == [],
                'Root must review the actual native runtime for scanner-only execution')
            bootstrap_report = json.loads(read(args.runtime_report, args.runtime_report_sha256, pins))
            terminal = json.loads(read(args.runtime_terminal, args.runtime_terminal_sha256, pins))
            require(terminal['status'] == bootstrap_report['status'] ==
                'runtime-discovered-awaiting-root-source-library-review' and
                terminal['report'] == str(Path(args.runtime_report).resolve()) and
                terminal['report_sha256'] == args.runtime_report_sha256 and
                terminal['inherited_common_mutex'] is True and
                bootstrap_report['terminal_evidence_complete'] is True and
                terminal['partial_evidence_pins'] == bootstrap_report['pins'] and
                not bootstrap_report.get('error') and not bootstrap_report.get('terminal_artifact_errors') and
                bootstrap_report.get('owned_mount_requires_root_review') is not True and
                bootstrap_report['attach_device_closure'] == 'exact-requested-mount-detached',
                'Actual bootstrap terminal and complete image/mount evidence required')
            require(review['runtime_report_sha256'] == args.runtime_report_sha256 and
                review['runtime_terminal_sha256'] == args.runtime_terminal_sha256 and
                review['binary'] == bootstrap_report['binary'] and
                review['binary_sha256'] == bootstrap_report['binary_sha256'] and
                review['environment'] == bootstrap_report['environment'] and
                bootstrap_report['architecture'] == 'arm64' and
                bootstrap_report['version_components'] == [10, 0, 2], 'Scanner review/runtime mismatch')
            for path, digest in bootstrap_report['pins'].items():
                read(path, digest, pins)
            for path, digest in review['fixture_source_pins'].items():
                read(path, digest, pins)
            required_sources = {str(path.resolve()) for path in (
                HERE/'foreign_prolog.py', HERE/'test_prolog.py', HERE/'scan_source.pl',
                HERE/'run_fixtures.py', HERE/'RUNNER_REVIEW.md', Path(sys.executable))}
            require(set(review['fixture_source_pins']) == required_sources,
                'Exact reviewed fixture source/Python inventory required')
            require(review['python_executable'] == str(Path(sys.executable).resolve()), 'Different reviewed Python')
            require(str(BOOTSTRAP.resolve()) in pins, 'Original bootstrap loader must be actually pinned')
            bootstrap = fresh(BOOTSTRAP, 'prolog_owner_fixture_bootstrap_v2', pins)
            preparation, sealed, pins = bootstrap.load_preparation(
                SimpleNamespace(proposal_sha256=bootstrap_report['proposal_sha256']), pins)
            service, _, guard, harness, pins = bootstrap.reused_service(preparation, sealed, pins)
            harness.install_interrupt_handlers()
            environment = dict(harness.compact.ENV)
            environment.update(review['environment'])
            binary = review['binary']
            read(binary, review['binary_sha256'], pins)
            unittest_original = Path(importlib.util.find_spec('unittest').origin).resolve().parent
            unittest_snapshot = output/'python/unittest'
            unittest_snapshot.mkdir(parents=True)
            for source in sorted(unittest_original.rglob('*.py')):
                target = unittest_snapshot/source.relative_to(unittest_original)
                target.parent.mkdir(parents=True, exist_ok=True)
                raw = source.read_bytes()
                digest = hashlib.sha256(raw).hexdigest()
                require(str(source) not in pins or pins[str(source)] == digest, 'Conflicting unittest source')
                target.write_bytes(raw)
                target.chmod(0o444)
                pins[str(source)] = pins[str(target)] = digest
            sys.path.insert(0, str(unittest_snapshot.parent))
            unittest = importlib.import_module('unittest')
            def runner_identities():
                identities = {}
                for name, module in list(sys.modules.items()):
                    if name == 'unittest' or name.startswith('unittest.'):
                        path = Path(module.__file__).resolve(strict=True)
                        require(path.is_relative_to(unittest_snapshot) and path.suffix == '.py' and
                            str(path) in pins, 'Unittest import escaped its actual frozen snapshot')
                        read(path, pins[str(path)], pins)
                        identities[name] = str(path)
                require(identities.get('unittest') == str(unittest_snapshot/'__init__.py'),
                    'Wrong unittest package origin')
                return identities
            runner_identities()
            projector = fresh(HERE/'foreign_prolog.py', 'foreign_prolog', pins)
            tests = fresh(HERE/'test_prolog.py', 'test_prolog', pins)
            original_project = tests.count_prolog

            def recorded_project(source_bytes, packet, **kwargs):
                result = original_project(source_bytes, packet, **kwargs)
                target = output/'projections'/str(len(projections)+1)
                target.mkdir(parents=True)
                (target/'original-source.pl').write_bytes(source_bytes)
                save(target/'packet.json', deepcopy(packet))
                save(target/'projection.json', result)
                projections.append(dict(test_id=started[-1], directory=str(target)))
                return result

            def owned_scan(command, **kwargs):
                require(kwargs == dict(stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
                    timeout=20, pass_fds=(lock.fileno(),)), 'Unexpected fixture subprocess policy')
                source = Path(command[-1]).resolve(strict=True)
                expected_command = [binary, '-q', '-f', 'none', '-l', str(HERE/'scan_source.pl'),
                    '-g', 'prolog_source_scan_v1:main', '--', str(source)]
                require(command == expected_command, 'Only the exact original reader invocation is authorized')
                target = output/'scanner-attempts'/str(len(scanner_attempts)+1)
                target.mkdir(parents=True)
                input_copy = target/'original-inputs'
                input_copy.mkdir()
                inputs = []
                for original in sorted(source.parent.glob('*.pl')):
                    raw = original.read_bytes()
                    (input_copy/original.name).write_bytes(raw)
                    inputs.append(dict(original_path=str(original), copied_path=str(input_copy/original.name),
                        sha256=hashlib.sha256(raw).hexdigest()))
                require(any(item['original_path'] == str(source) for item in inputs), 'Original fixture input absent')
                save(target/'inputs.json', dict(test_id=started[-1], original_inputs=inputs))
                scanner_attempts.append(dict(test_id=started[-1], directory=str(target), command=command))
                try:
                    result = service.owned_command(guard, harness, command, environment,
                        target, lock.fileno(), 20, 'scan')
                    detail, stdout, stderr = result['detail'], result['stdout'], result['stderr']
                except AssertionError:
                    # Syntax-error fixtures need their real nonzero reader
                    # status. No timeout, supervisor or cleanup error converts
                    # into an expected parser error. Preserve the service's
                    # original exception/receipt, even for these two cases.
                    detail = json.loads((target/'scan.json').read_bytes())
                    require(detail.get('error_type') == 'AssertionError' and
                        detail.get('error', '').startswith("('Owned command failed', 'scan', ") and
                        type(detail.get('exit_status')) is int and detail['exit_status'] != 0 and
                        type(detail.get('final_exit_status')) is int and
                        detail['final_exit_status'] == detail['exit_status'] and
                        type(detail.get('pid')) is int and detail['pid'] > 0 and
                        not detail.get('cleanup_errors') and not harness.group_members(detail['pid']),
                        'Nonzero reader status is not a clean owned parser-error result')
                    stdout = (target/'scan.stdout').read_bytes()
                    stderr = (target/'scan.stderr').read_bytes()
                return subprocess.CompletedProcess(command, detail['exit_status'], stdout, stderr)

            # Replace references only inside this freshly loaded fixture
            # module. Original bodies and the process-wide subprocess module
            # remain unchanged; the projection observer returns exact results.
            tests.subprocess = SimpleNamespace(PIPE=subprocess.PIPE, run=owned_scan)
            tests.count_prolog = recorded_project
            os.environ['PROLOG_SCANNER_RUNTIME'] = binary
            os.environ['PROLOG_SCANNER_LOCK_FD'] = str(lock.fileno())
            suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
            def identities(items):
                for item in items:
                    if isinstance(item, unittest.TestSuite): yield from identities(item)
                    else: yield item.id()
            discovered = list(identities(suite))
            report['discovered_ids'] = list(discovered)
            runner_identities()
            require(len(discovered) == len(set(discovered)) == 31 and set(discovered) == set(EXPECTED),
                'Exactly the reviewed thirty-one fixture IDs must be discovered')
            class Result(unittest.TextTestResult):
                def __init__(self, *args, **kwargs):
                    nonlocal live_result
                    super().__init__(*args, **kwargs)
                    live_result = self
                def startTest(self, test):
                    started.append(test.id())
                    super().startTest(test)
            with (output/'fixtures.log').open('x') as stream:
                result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=Result).run(suite)
            complete = (result.wasSuccessful() and result.testsRun == 31 and
                len(started) == len(set(started)) == 31 and set(started) == set(EXPECTED) and
                not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses)
            report.update(status='passed-awaiting-original-source-review' if complete else 'failed-fixtures',
                tests_run=result.testsRun, discovered_ids=discovered, started_ids=started,
                failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
                expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                exact_test_execution=len(started) == len(set(started)) == 31 and set(started) == set(EXPECTED))
            require(complete, 'Actual reader/projector fixtures did not all pass')
            report['runner_module_identities'] = runner_identities()
        except BaseException as failure:
            error = failure
            report.update(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            (output/'exception.log').write_text(traceback.format_exc())
        finally:
            report.update(scanner_attempts=scanner_attempts, projections=projections,
                discovered_ids=list(discovered), started_ids=list(started))
            if live_result is not None:
                report.update(tests_run=live_result.testsRun, failures=len(live_result.failures),
                    errors=len(live_result.errors), skipped=len(live_result.skipped),
                    expected_failures=len(live_result.expectedFailures),
                    unexpected_successes=len(live_result.unexpectedSuccesses))
            integrity_errors = []
            if 'runner_identities' in locals():
                try: report['runner_module_identities'] = runner_identities()
                except BaseException as failure:
                    integrity_errors.append(dict(stage='unittest-module-origin', error=str(failure)))
            for path, digest in pins.items():
                try:
                    require(hashlib.sha256(Path(path).read_bytes()).hexdigest() == digest, 'Changed fixture input')
                except BaseException as failure:
                    integrity_errors.append(dict(path=path, error=str(failure)))
            artifact_pins = {}
            for path in sorted(output.rglob('*')):
                if path.is_file():
                    try: artifact_pins[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
                    except BaseException as failure:
                        integrity_errors.append(dict(stage='partial-artifact-pin', path=str(path),
                            error=str(failure) or type(failure).__name__))
            report.update(input_pins=pins, pin_reconciliation_passed=not integrity_errors,
                terminal_artifact_errors=integrity_errors,
                available_bytes_not_fully_pinned=bool(integrity_errors))
            if integrity_errors: report['status'] = 'failed-terminal-integrity'
            report['artifact_pins'] = artifact_pins
            save(output/'fixture-receipt.json', report)
            save(output/'terminal-pin.json', dict(status=report['status'], shared_timing_mutex_held=True,
                receipt=str(output/'fixture-receipt.json'),
                receipt_sha256=hashlib.sha256((output/'fixture-receipt.json').read_bytes()).hexdigest(),
                artifact_pins=artifact_pins, input_pins=pins, automatic_retry=False,
                integrity_errors=integrity_errors, available_bytes_not_fully_pinned=bool(integrity_errors),
                source_counting_started=False, chart_admission=False))
        require(error is None and report['status'] == 'passed-awaiting-original-source-review',
            'Preserved failed fixture run; no automatic retry')
        print(json.dumps(dict(status=report['status'], tests_run=report['tests_run'],
            scanner_processes=len(scanner_attempts), chart_admission=False)), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('runtime-review', 'runtime-review-sha256', 'runtime-report', 'runtime-report-sha256',
                 'runtime-terminal', 'runtime-terminal-sha256', 'output'):
        parser.add_argument('--'+name, required=True)
    run(parser.parse_args())
