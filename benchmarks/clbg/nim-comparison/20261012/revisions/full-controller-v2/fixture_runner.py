"""Execute the frozen fixture inventory and retain actual test IDs/count/no skips."""
import argparse
import json
from pathlib import Path
import sys
import unittest

CONTROLLER = [
    'test_full_study.FailureIsolation.test_guard_admitted_runtime_output_or_resource_failure_can_continue_other_profiles',
    'test_full_study.FailureIsolation.test_frozen_guard_rejection_is_never_relabelled_source_failure',
    'test_full_study.FailureIsolation.test_every_ownership_environment_workload_or_pin_gate_is_literal_true',
    'test_full_study.FailureIsolation.test_exact_twenty_key_inventory_and_boolean_types_are_required',
]
INVENTORIES = {'controller': CONTROLLER}
MODULES = {'controller': ['test_full_study']}


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args):
        super().__init__(*args)
        self.actual_test_ids = []
    def startTest(self, test):
        self.actual_test_ids.append(test.id())
        super().startTest(test)


def validate_receipt(receipt, inventory):
    expected = INVENTORIES[inventory]
    assert receipt['status'] == 'passed'
    assert receipt['inventory'] == inventory and receipt['expected_test_ids'] == expected
    assert type(receipt['tests_run']) is int and receipt['tests_run'] == len(expected)
    assert receipt['discovered_tests'] == len(expected)
    actual = receipt['actual_test_ids']
    assert len(actual) == len(set(actual)) == len(expected) and set(actual) == set(expected)
    assert receipt['skipped'] == receipt['failures'] == receipt['errors'] == []
    assert receipt['expected_failures'] == receipt['unexpected_successes'] == []
    assert receipt['successful'] is True


def run(inventory, path):
    expected = INVENTORIES[inventory]
    receipt = dict(status='failed', inventory=inventory, expected_test_ids=expected,
                   actual_test_ids=[], tests_run=0, discovered_tests=0, successful=False)
    try:
        suite = unittest.defaultTestLoader.loadTestsFromNames(MODULES[inventory])
        receipt['discovered_tests'] = suite.countTestCases()
        result = unittest.TextTestRunner(verbosity=2, resultclass=RecordedResult).run(suite)
        receipt.update(actual_test_ids=result.actual_test_ids, tests_run=result.testsRun,
            successful=result.wasSuccessful(), skipped=[(test.id(), why) for test, why in result.skipped],
            failures=[(test.id(), detail) for test, detail in result.failures],
            errors=[(test.id(), detail) for test, detail in result.errors],
            expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
            unexpected_successes=[test.id() for test in result.unexpectedSuccesses])
        receipt['status'] = 'passed'
        validate_receipt(receipt, inventory)
    except BaseException as error:
        receipt.update(status='failed', inventory_error=str(error) or type(error).__name__,
                       inventory_error_type=type(error).__name__)
    finally:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('x') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
    return 0 if receipt['status'] == 'passed' else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--set', choices=tuple(INVENTORIES), required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    sys.exit(run(args.set, args.receipt))


if __name__ == '__main__':
    main()
