"""Execute the frozen fixture inventory and retain actual test IDs/count/no skips."""
import argparse
import json
from pathlib import Path
import sys
import unittest

OWNERSHIP = ['test_guarded_prepare_v6.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup', 'test_guarded_prepare_v6.OwnershipChecks.test_identity_failure_cleans_known_child', 'test_guarded_prepare_v6.OwnershipChecks.test_journal_failure_cleans_known_child', 'test_guarded_prepare_v6.OwnershipChecks.test_timeout_preserves_complete_drained_output', 'test_guarded_prepare_v6.OwnershipChecks.test_orphan_compiler_child_is_fatal', 'test_guarded_prepare_v6.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits', 'test_guarded_prepare_v6.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group', 'test_guarded_prepare_v6.ProbeRestorationChecks.test_setup_failure_restores_present_state', 'test_guarded_prepare_v6.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure', 'test_guarded_prepare_v6.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes', 'test_guarded_prepare_v6.ProbeRestorationChecks.test_trace_failure_archives_and_stops', 'test_compat_reuse_v5.ReuseBoundary.test_all_successes_retained_and_failure_not_admitted', 'test_compat_reuse_v5.ReuseBoundary.test_missing_duplicate_or_changed_fingerprint_rejected', 'test_compat_reuse_v5.ReuseBoundary.test_direct_resource_power_status_and_group_failures_rejected', 'test_compat_reuse_v5.ReuseBoundary.test_failed_primitive_stays_failed', 'test_compat_reuse_v5.ReuseBoundary.test_signed_library_contract_matches_documented_floor_and_separate_gmp_truncation', 'test_compat_reuse_v5.ActualPiTrace.test_both_source_copies_keep_actual_divisions_and_unique_anchors', 'test_compat_reuse_v5.ActualPiTrace.test_all_candidate_operands_and_rejected_steps_reconcile', 'test_compat_reuse_v5.ActualPiTrace.test_missing_wrong_or_negative_operands_rejected', 'test_primitive_reuse_v6.PrimitiveReuse.test_exact_three_prior_records_retained_without_relabelling', 'test_primitive_reuse_v6.PrimitiveReuse.test_missing_duplicate_changed_identity_or_nonboolean_gate_rejected', 'test_primitive_reuse_v6.PrimitiveReuse.test_direct_native_ownership_environment_and_observation_corruptions_rejected', 'test_primitive_reuse_v6.PrimitiveReuse.test_only_unscored_reserved_diagnostic_identifier_changes']
INVENTORIES = {'ownership': OWNERSHIP}
MODULES = {'ownership': ['test_guarded_prepare_v6','test_compat_reuse_v5','test_primitive_reuse_v6']}

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
