"""Source-only direct immutable fixture worker; no benchmark source replay."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import sys
import traceback
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = ('gleam_reader.py', 'gleam_units.py', 'test_gleam_reader.py',
         'test_gleam_units.py', 'PLAN.md', 'OWNER_REVIEW.md', 'SCALAR_CALL_REVIEW.md')
EXPECTED_IDS = tuple('test_gleam_reader.GleamOwners.' + method for method in (
    'test_import_selectors_and_namespace_aliases_are_semantic_imports',
    'test_packed_and_expanded_independent_let_bindings_have_two_owners',
    'test_function_signature_formals_and_type_returns_are_structural',
    'test_external_annotation_and_bodyless_ffi_declaration_have_real_owners',
    'test_generic_external_types_and_custom_constructor_fields_are_structural',
    'test_packed_case_arm_patterns_headers_and_body_values_remain_distinct',
    'test_nested_case_initializer_and_arm_block_keep_real_binding_work',
    'test_callback_body_inside_constructor_argument_cannot_hide_two_effects',
    'test_nested_worker_callbacks_and_dict_maps_remain_gleam_owned',
    'test_let_assert_destructuring_is_one_binding_from_one_producer',
    'test_tuple_projection_list_spread_and_record_update_arguments_are_structural',
    'test_bit_array_utf8_bytes_and_default_byte_segments_have_owned_expressions',
    'test_scalar_grouping_braces_are_visible_for_owner_review_not_assumed_data',
    'test_pipeline_composition_preserves_nested_anonymous_body',
    'test_comments_strings_unicode_and_exact_source_spans_remain_separate',
    'test_unknown_annotation_guard_use_and_malformed_forms_diagnose',
    'test_const_literal_data_never_hides_function_or_runtime_call_work',
)) + tuple('test_gleam_units.GleamUnitProjection.' + method for method in (
    'test_packed_and_expanded_independent_bindings_and_body_agree',
    'test_let_case_prefix_combines_once_retaining_every_arm_body_close',
    'test_distinct_callbacks_never_share_a_header_or_written_close',
    'test_separate_preceding_effect_is_not_combined_with_later_callback',
    'test_packed_callback_bindings_effects_tail_and_prefix_all_survive',
    'test_numeric_grouping_remains_one_outer_expression',
    'test_nested_block_cannot_hide_multiple_bindings_effects_or_tail',
    'test_unproved_earlier_callable_operand_is_not_silently_merged',
    'test_literal_data_wrapping_and_comment_exclusion_remain_explicit',
    'test_type_constructor_headers_and_ffi_directives_are_counted',
    'test_unsupported_source_never_falls_back_to_a_physical_count',
))


def now(): return datetime.now(timezone.utc).isoformat()


def save_new(path, payload):
    with path.open('x') as stream:
        json.dump(payload, stream, indent=2); stream.write('\n')


def state(job, status, **fields):
    temporary = job / 'state.json.tmp'
    temporary.write_text(json.dumps(dict(status=status, pid=os.getpid(),
        updated_at=now(), automatic_retry=False, **fields), indent=2) + '\n')
    temporary.replace(job / 'state.json')


def freeze_and_exec(job):
    job = job.resolve(); job.mkdir(parents=True, exist_ok=True)
    with (job / 'freeze-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not (job / 'launch.json').exists() and not (job / 'snapshot').exists()
        snapshot = job / 'snapshot'; snapshot.mkdir()
        for name in FILES:
            shutil.copyfile(HERE / name, snapshot / name); (snapshot / name).chmod(0o444)
        snapshot.chmod(0o555)
        runner = job / 'worker.py'; shutil.copyfile(Path(__file__), runner); runner.chmod(0o444)
        command = [sys.executable, '-B', '-u', str(runner), '--worker', str(job)]
        save_new(job / 'launch.json', dict(status='frozen-direct-fixture-worker',
            frozen_at=now(), source_packet=str(HERE), snapshot=str(snapshot),
            snapshot_files=list(FILES), expected_ids=list(EXPECTED_IDS), expected_count=28,
            command=command, pid=os.getpid(), pgid=os.getpgrp(), same_pid_exec=True,
            source_counting_started=False, native_probes=0, chart_admission=False, automatic_retry=False))
        print('Frozen Gleam fixture packet; exec immutable worker in same PID.', flush=True)
        try:
            os.execv(sys.executable, command)
        except BaseException as error:
            save_new(job / 'exec-failure.json', dict(status='failed-stop', error=str(error),
                error_type=type(error).__name__, detached_children_started=0, automatic_retry=False))
            raise


def worker(job):
    with (job / 'worker-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not (job / 'fixture-receipt.json').exists()
        state(job, 'waiting-for-common-mutex')
        print('WAIT LOCK: exact28 Gleam source-owner fixtures', flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            receipt = dict(status='failed-stop', started_at=now(), actual_common_mutex_held=True,
                source_counting_started=False, benchmark_processes_started=0,
                fixture_native_probes=0, chart_admission=False, automatic_retry=False,
                pid=os.getpid(), pgid=os.getpgrp(), expected_ids=list(EXPECTED_IDS), expected_count=28,
                python_executable=sys.executable, python_version=sys.version)
            terminal_status = 'failed-stop'
            try:
                import hashlib
                import importlib
                import sysconfig
                assert sys.version_info[:2] == (3, 14)
                descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
                assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
                snapshot = job / 'snapshot'
                assert {path.name for path in snapshot.iterdir()} == set(FILES)
                unittest_root = Path(sysconfig.get_path('stdlib')) / 'unittest'
                paths = [snapshot / name for name in FILES] + [job / 'worker.py', job / 'launch.json', Path(sys.executable).resolve()]
                paths += sorted(unittest_root.rglob('*.py'))
                def pins(): return {str(path.resolve()): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
                before = pins(); receipt['pins_before'] = before
                import unittest
                assert Path(unittest.__file__).resolve().parent == unittest_root.resolve()
                sys.path.insert(0, str(snapshot))
                modules = [importlib.import_module(name) for name in ('test_gleam_reader', 'test_gleam_units')]
                for name in ('gleam_reader', 'gleam_units', 'test_gleam_reader', 'test_gleam_units'):
                    assert Path(sys.modules[name].__file__).resolve() == snapshot / (name + '.py')
                suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(module) for module in modules)
                def ids(tests):
                    for test in tests:
                        if isinstance(test, unittest.TestSuite): yield from ids(test)
                        else: yield test.id()
                discovered = list(ids(suite))
                assert len(EXPECTED_IDS) == len(set(EXPECTED_IDS)) == 28
                assert suite.countTestCases() == 28 and sorted(discovered) == sorted(EXPECTED_IDS)
                class StrictResult(unittest.TextTestResult):
                    def __init__(self, *args, **kwargs):
                        super().__init__(*args, **kwargs); self.started_ids = []
                    def startTest(self, test):
                        self.started_ids.append(test.id()); super().startTest(test)
                state(job, 'actual-fixtures-running', expected_count=28)
                raw = job / 'fixtures.log'
                with raw.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=StrictResult).run(suite)
                exact = result.testsRun == 28 and len(result.started_ids) == 28 and sorted(result.started_ids) == sorted(EXPECTED_IDS)
                passed = result.wasSuccessful() and exact and not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses
                after = pins(); assert before == after
                receipt.update(status='passed-awaiting-root-source-review' if passed else 'failed-fixtures',
                    tests_run=result.testsRun, discovered_ids=discovered, started_ids=result.started_ids,
                    failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
                    expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                    pins_after=after, pin_reconciliation_passed=True,
                    raw_log=str(raw), raw_log_sha256=hashlib.sha256(raw.read_bytes()).hexdigest(),
                    python_executable=sys.executable, python_version=sys.version, finished_at=now())
                save_new(job / 'fixture-receipt.json', receipt)
                terminal_status = receipt['status']
                state(job, receipt['status'], tests_run=result.testsRun)
                print(json.dumps(dict(status=receipt['status'], tests_run=result.testsRun,
                    failures=len(result.failures), errors=len(result.errors))), flush=True)
                return 0 if passed else 1
            except BaseException as error:
                terminal_status = 'failed-stop'
                detail = traceback.format_exc()
                print(detail, file=sys.stderr, flush=True)
                with (job / 'exception.log').open('x') as stream:
                    stream.write(detail)
                receipt.update(status='failed-stop', error=str(error), error_type=type(error).__name__, finished_at=now())
                if not (job / 'fixture-receipt.json').exists(): save_new(job / 'fixture-receipt.json', receipt)
                state(job, 'failed-stop', error=str(error))
                return 1
            finally:
                # Pin every actually available terminal/raw artifact, including
                # exceptions after fixture execution or receipt persistence.
                # Remain under the same acquired common mutex for these hashes.
                import hashlib
                receipt_path = job / 'fixture-receipt.json'
                if receipt_path.exists():
                    available = [receipt_path] + [path for path in (job / 'fixtures.log', job / 'exception.log') if path.exists()]
                    try:
                        save_new(job / 'fixture-receipt-pin.json', dict(actual_common_mutex_held=True,
                            terminal_worker_status=terminal_status,
                            saved_receipt_status=json.loads(receipt_path.read_text())['status'],
                            terminal_artifact_pins={str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in available},
                            receipt=str(receipt_path), receipt_sha256=hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
                            chart_admission=False, automatic_retry=False))
                    except BaseException as pin_error:
                        state(job, 'failed-terminal-artifact-pin', error=str(pin_error))
                        raise


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--direct', action='store_true'); modes.add_argument('--worker', type=Path)
    parser.add_argument('--job', type=Path, default=HERE / 'jobs/fixtures-v1')
    args = parser.parse_args()
    if args.worker: return worker(args.worker.resolve())
    freeze_and_exec(args.job)
    return 1


if __name__ == '__main__': raise SystemExit(main())
