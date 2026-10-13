"""Twenty-three fixture-only cases; no formatter or application authorization."""
import argparse
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import traceback

PACKET = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = ('measure.py', 'load_evidence.py', 'overlay.py', 'endpoints.py', 'test_measure.py',
         'test_overlay.py', 'test_endpoints.py', 'qualify_fixtures.py', 'FORMATTERS.md', 'LEGACY_FORMATTERS.md', 'REVIEW.md')
IDS = [
    'test_endpoints.FormattedEndpoints.test_third_source_is_promoted_before_pruning_with_unchanged_canonical_baselines',
    'test_endpoints.FormattedEndpoints.test_absent_and_unavailable_fields_keep_the_canonical_pool_with_nulls',
    'test_endpoints.FormattedEndpoints.test_formatted_baseline_uses_only_complete_qualified_sources',
    'test_endpoints.FormattedEndpoints.test_partial_file_receipt_has_no_count_and_does_not_supply_a_formatted_baseline',
    'test_endpoints.FormattedEndpoints.test_formatted_fast_can_differ_without_changing_original_fast_or_time',
    'test_endpoints.FormattedEndpoints.test_source_only_formatted_small_has_no_invented_timing_or_complete_suite',
    'test_endpoints.FormattedEndpoints.test_stable_formatted_ties_and_single_source_union',
    'test_endpoints.FormattedEndpoints.test_late_corrupt_formatted_record_rejects_the_whole_transaction',
    'test_endpoints.FormattedEndpoints.test_late_canonical_failure_still_rejects_the_whole_transaction',
    'test_endpoints.FormattedEndpoints.test_v_and_red_cannot_become_visualized_candidates',
    'test_endpoints.FormattedEndpoints.test_formatted_selection_preserves_saved_canonical_baselines_and_coordinates',
    'test_endpoints.FormattedEndpoints.test_full_inventory_survives_endpoint_pruning_with_source_closure',
    'test_endpoints.FormattedEndpoints.test_inventory_disagreements_and_excluded_rows_reject_transactionally',
    'test_overlay.FormattedOverlay.test_actual_measured_execution_fields_and_independent_receipt_are_required',
    'test_overlay.FormattedOverlay.test_whole_bundle_requires_every_authored_file_and_accepts_a_zero_line_helper',
    'test_overlay.FormattedOverlay.test_unavailable_helper_keeps_bundle_null_without_main_file_fallback',
    'test_overlay.FormattedOverlay.test_measurement_and_source_identities_are_exact_while_new_metadata_is_preserved',
    'test_overlay.FormattedOverlay.test_late_receipt_pin_terminal_and_selection_failures_are_transactional',
    'test_overlay.FormattedOverlay.test_inventory_only_and_shared_profiles_bind_exact_sources_and_measurements',
    'test_measure.FormatterBoundary.test_only_isolated_source_placeholders_and_literal_argv',
    'test_measure.FormatterBoundary.test_import_origins_reject_unpinned_external_and_installed_packages',
    'test_measure.FormatterBoundary.test_physical_lines_do_not_call_semantic_normalization',
    'test_measure.FormatterBoundary.test_process_success_requires_literal_mutex_and_complete_cleanup',
]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        if sha(path) != digest:
            raise ValueError('Changed fixture input: ' + path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    if not args.revision or not all(c.isalnum() or c in '-_' for c in args.revision):
        raise ValueError('Invalid fresh fixture revision')
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        if (opened.st_dev, opened.st_ino) != (named.st_dev, named.st_ino):
            raise ValueError('Common mutex inode differs')
        output = PACKET / 'jobs' / args.revision
        if not output.resolve().is_relative_to(PACKET.resolve()) or output.exists():
            raise ValueError('Fixture output must be fresh and contained')
        output.mkdir(parents=True)
        pins = {}; body_error = None; result = None
        receipt = dict(status='not-started', expected_test_ids=IDS, discovered_test_ids=[], actual_test_ids=[],
            discovered_tests=0, tests_run=0, failures=[], errors=[], skipped=[], expected_failures=[], unexpected_successes=[],
            actual_native_subjects=0, actual_signals_sent=0, actual_formatter_processes=0,
            inherited_common_mutex=True, chart_admission=False)
        try:
            if not sys.dont_write_bytecode or any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules):
                raise ValueError('Use a fresh Python -B process with no cached unittest')
            snapshot = output / 'source'; snapshot.mkdir()
            for name in FILES:
                original, target = PACKET / name, snapshot / name
                raw = original.read_bytes(); target.write_bytes(raw); target.chmod(0o444)
                pins[str(original)] = pins[str(target)] = hashlib.sha256(raw).hexdigest()
            python = str(Path(sys.executable).resolve())
            pins[python] = sha(python)
            original_unittest = Path(importlib.util.find_spec('unittest').origin).resolve().parent
            frozen = output / 'python/unittest'; frozen.mkdir(parents=True)
            for original in sorted(original_unittest.rglob('*.py')):
                target = frozen / original.relative_to(original_unittest)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(original, target); target.chmod(0o444)
                pins[str(original)] = pins[str(target)] = sha(original)
            verify(pins)
            sys.path[:0] = [str(snapshot), str(frozen.parent)]
            unittest = importlib.import_module('unittest')
            if Path(unittest.__file__).resolve() != frozen / '__init__.py':
                raise ValueError('Unittest did not load the immutable snapshot')
            class Recorded(unittest.TextTestResult):
                def __init__(self, *args):
                    nonlocal result
                    super().__init__(*args)
                    result = self
                def startTest(self, test):
                    receipt['actual_test_ids'].append(test.id())
                    super().startTest(test)
            loader = unittest.TestLoader()
            suite = unittest.TestSuite(loader.loadTestsFromName(name) for name in ('test_endpoints','test_overlay','test_measure'))
            def collect(part):
                if isinstance(part, unittest.TestSuite):
                    for child in part: collect(child)
                else: receipt['discovered_test_ids'].append(part.id())
            collect(suite)
            receipt['discovered_tests'] = suite.countTestCases()
            receipt['loader_errors'] = list(loader.errors)
            save(output / 'discovery.json', dict(loader_errors=loader.errors,
                discovered_test_ids=receipt['discovered_test_ids'], sys_path=sys.path[:]))
            if loader.errors or len(receipt['discovered_test_ids']) != 23 or set(receipt['discovered_test_ids']) != set(IDS):
                raise ValueError('Exact twenty-three fixture discovery failed; actual loader errors saved')
            verify(pins)
            with (output / 'fixtures.log').open('x') as stream:
                result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=Recorded).run(suite)
            receipt.update(tests_run=result.testsRun, failures=[(t.id(), why) for t, why in result.failures],
                errors=[(t.id(), why) for t, why in result.errors], skipped=[(t.id(), why) for t, why in result.skipped],
                expected_failures=[(t.id(), why) for t, why in result.expectedFailures],
                unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
            for name, loaded in list(sys.modules.items()):
                path = getattr(loaded, '__file__', None)
                if name == 'unittest' or name.startswith('unittest.'):
                    actual = Path(path).resolve()
                    if not actual.is_relative_to(frozen) or pins.get(str(actual)) != sha(actual):
                        raise ValueError('Unittest module/source identity differs')
                elif name in ('test_endpoints','test_overlay','test_measure','overlay','endpoints','measure','load_evidence'):
                    actual = Path(path).resolve()
                    if actual != snapshot / (name + '.py') or pins.get(str(actual)) != sha(actual):
                        raise ValueError('Fixture module/source identity differs')
            if result.testsRun != 23 or len(receipt['actual_test_ids']) != 23 or len(set(receipt['actual_test_ids'])) != 23:
                raise ValueError('Actual fixture start inventory differs')
            if set(receipt['actual_test_ids']) != set(IDS) or any(receipt[key] for key in
                    ('failures','errors','skipped','expected_failures','unexpected_successes')) or not result.wasSuccessful():
                raise ValueError('Fixture outcomes are not a complete zero-failure pass')
            verify(pins); receipt['status'] = 'passed-formatted-loc-fixtures'
        except BaseException as failure:
            body_error = dict(error=str(failure), error_type=type(failure).__name__)
            receipt['status'] = 'failed-formatted-loc-fixtures'
            try:
                with (output / 'exception.log').open('x') as stream: traceback.print_exc(file=stream)
            except BaseException as journal_error:
                receipt['exception_journal_error'] = str(journal_error)
            raise
        finally:
            if result is not None:
                receipt.update(tests_run=result.testsRun, failures=[(t.id(), why) for t, why in result.failures],
                    errors=[(t.id(), why) for t, why in result.errors], skipped=[(t.id(), why) for t, why in result.skipped],
                    expected_failures=[(t.id(), why) for t, why in result.expectedFailures],
                    unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
            save(output / 'fixture-receipt.json', receipt)
            errors = []; partial = dict(pins)
            for path in sorted(output.rglob('*')):
                if path.is_file() and path.name != 'terminal.json':
                    try:
                        name, digest = str(path.resolve()), sha(path)
                        if name in partial and partial[name] != digest:
                            raise ValueError('Previously pinned fixture byte changed')
                        partial[name] = digest
                    except BaseException as failure: errors.append(dict(path=str(path), error=str(failure)))
            for path, digest in partial.items():
                try:
                    if sha(path) != digest: raise ValueError('Changed fixture/source bytes')
                except BaseException as failure: errors.append(dict(path=path, error=str(failure)))
            save(output / 'terminal.json', dict(status='failed-terminal-integrity' if errors else receipt['status'],
                body_error=body_error, integrity_errors=errors, available_bytes_not_fully_pinned=bool(errors),
                receipt=str(output / 'fixture-receipt.json'), receipt_sha256=sha(output / 'fixture-receipt.json'),
                pins=partial, inherited_common_mutex=True, chart_admission=False,
                actual_native_subjects=0, actual_formatter_processes=0))
            if errors: raise RuntimeError('Fixture terminal integrity failed; partial readable evidence retained')


if __name__ == '__main__':
    main()
