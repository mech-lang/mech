"""Exact actual fixture inventory; run only by the common-mutex seal owner."""
import argparse
import fcntl
import json
from pathlib import Path
import sys
import os
import unittest

EXPECTED = [
    'test_guarded_prepare.CandidateInventory.test_missing_helper_or_substituted_profile_is_rejected',
    'test_guarded_prepare.CandidateInventory.test_runtime_classpath_has_only_classes_and_reviewed_vendor_jars',
    'test_guarded_prepare.CandidateInventory.test_compiler_expression_quotes_paths_and_binds_exact_two_namespaces',
    'test_guarded_prepare.CandidateInventory.test_exact_twenty_true_gates_reject_bool_like_missing_and_extra',
    'test_guarded_prepare.CandidateInventory.test_exact_two_nbody_source_aliases_preserve_all_other_scored_inputs',
    'test_guarded_prepare.CandidateInventory.test_reuse_requires_every_original24_case_binding_and_exact_safety_gate',
]


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args):
        super().__init__(*args)
        self.actual_test_ids = []
    def startTest(self, test):
        self.actual_test_ids.append(test.id())
        super().startTest(test)


def validate_receipt(receipt):
    assert receipt['status'] == 'passed' and receipt['expected_test_ids'] == EXPECTED
    assert type(receipt['tests_run']) is int and receipt['tests_run'] == len(EXPECTED)
    assert receipt['discovered_tests'] == len(EXPECTED)
    assert sorted(receipt['discovered_test_ids']) == sorted(EXPECTED)
    assert len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == len(EXPECTED)
    assert set(receipt['actual_test_ids']) == set(EXPECTED)
    assert receipt['successful'] is True
    assert receipt['skipped'] == receipt['failures'] == receipt['errors'] == []
    assert receipt['expected_failures'] == receipt['unexpected_successes'] == []
    assert receipt['native_subjects'] == receipt['actual_signals'] == receipt['downloads'] == 0
    assert receipt['inherited_common_mutex'] is True and receipt['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock'


def run(path):
    receipt = dict(status='failed', expected_test_ids=EXPECTED, actual_test_ids=[],
        tests_run=0, discovered_tests=0, successful=False,
        native_subjects=0, actual_signals=0, downloads=0)
    try:
        descriptor = int(os.environ['CLBG_MUTEX_FD'])
        named, opened = Path('/tmp/mech-clbg-timing.lock').stat(), os.fstat(descriptor)
        assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        receipt.update(inherited_common_mutex=True, shared_timing_lock='/tmp/mech-clbg-timing.lock')
        suite = unittest.defaultTestLoader.loadTestsFromName('test_guarded_prepare')
        def flatten(value):
            for test in value:
                if isinstance(test, unittest.TestSuite):
                    yield from flatten(test)
                else:
                    yield test.id()
        receipt['discovered_test_ids'] = list(flatten(suite))
        receipt['discovered_tests'] = suite.countTestCases()
        result = unittest.TextTestRunner(verbosity=2, resultclass=RecordedResult).run(suite)
        receipt.update(actual_test_ids=result.actual_test_ids, tests_run=result.testsRun,
            successful=result.wasSuccessful(), skipped=[(test.id(), why) for test, why in result.skipped],
            failures=[(test.id(), detail) for test, detail in result.failures],
            errors=[(test.id(), detail) for test, detail in result.errors],
            expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
            unexpected_successes=[test.id() for test in result.unexpectedSuccesses], status='passed')
        validate_receipt(receipt)
    except BaseException as error:
        receipt.update(status='failed', error=str(error) or type(error).__name__, error_type=type(error).__name__)
    finally:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('x') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
    return 0 if receipt['status'] == 'passed' else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    sys.exit(run(args.receipt))


if __name__ == '__main__':
    main()
