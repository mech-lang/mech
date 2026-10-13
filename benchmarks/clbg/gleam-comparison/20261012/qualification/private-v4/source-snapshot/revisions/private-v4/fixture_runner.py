"""Retain the actual nine-method preparation inventory with zero skipped tests."""
import argparse
import json
from pathlib import Path
import unittest

INVENTORY = [
    'test_guarded_prepare.CandidatePreparation.test_preheader_input_is_declared_and_reference_ignores_preamble',
    'test_guarded_prepare.CandidatePreparation.test_complete_authored_inventory_and_explicit_beam_command',
    'test_guarded_prepare.CandidatePreparation.test_missing_form_foreign_source_and_ffi_module_collision_are_rejected',
    'test_guarded_prepare.CandidatePreparation.test_hex_inner_checksum_and_exact_outer_inventory_are_required',
    'test_guarded_prepare.CandidatePreparation.test_archive_traversal_and_links_cannot_become_compiler_inputs',
    'test_guarded_prepare.AdditiveRepair.test_exact_old_compact_tree_records_are_retained_without_mutation',
    'test_guarded_prepare.AdditiveRepair.test_changed_missing_or_nonboolean_old_record_is_rejected',
    'test_guarded_prepare.AdditiveRepair.test_exact_six_grouping_pairs_change_only_the_five_reviewed_sources',
    'test_guarded_prepare.AdditiveRepair.test_saved_fingerprint_projection_is_exact_and_runtime_pins_stay_separate',
]


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args):
        super().__init__(*args)
        self.actual_test_ids = []
    def startTest(self, test):
        self.actual_test_ids.append(test.id())
        super().startTest(test)


def validate_receipt(value):
    assert value['status'] == 'passed' and value['expected_test_ids'] == INVENTORY
    assert type(value['tests_run']) is int and value['tests_run'] == value['discovered_tests'] == len(INVENTORY)
    assert len(value['actual_test_ids']) == len(set(value['actual_test_ids'])) == len(INVENTORY)
    assert set(value['actual_test_ids']) == set(INVENTORY)
    assert value['successful'] is True
    assert value['skipped'] == value['errors'] == value['failures'] == []
    assert value['expected_failures'] == value['unexpected_successes'] == []


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    receipt = dict(status='failed', expected_test_ids=INVENTORY)
    try:
        suite = unittest.defaultTestLoader.loadTestsFromNames(['test_guarded_prepare'])
        discovered = suite.countTestCases()
        result = unittest.TextTestRunner(verbosity=2, resultclass=RecordedResult).run(suite)
        receipt.update(status='passed', tests_run=result.testsRun, discovered_tests=discovered,
            actual_test_ids=result.actual_test_ids, successful=result.wasSuccessful(),
            skipped=[(test.id(), why) for test, why in result.skipped],
            errors=[(test.id(), detail) for test, detail in result.errors],
            failures=[(test.id(), detail) for test, detail in result.failures],
            expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
            unexpected_successes=[test.id() for test in result.unexpectedSuccesses])
        validate_receipt(receipt)
    except BaseException as error:
        receipt.update(status='failed', inventory_error=str(error) or type(error).__name__)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    with args.receipt.open('x') as stream:
        json.dump(receipt, stream, indent=2)
        stream.write('\n')
    raise SystemExit(0 if receipt['status'] == 'passed' else 1)


if __name__ == '__main__':
    main()
