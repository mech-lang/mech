"""Execute the frozen fixture inventory and retain actual test IDs/count/no skips."""
import argparse
import json
from pathlib import Path
import sys
import unittest

OWNERSHIP = [
    'test_guarded_prepare.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup',
    'test_guarded_prepare.OwnershipChecks.test_identity_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_journal_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_timeout_preserves_complete_drained_output',
    'test_guarded_prepare.OwnershipChecks.test_orphan_compiler_child_is_fatal',
    'test_guarded_prepare.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits',
    'test_guarded_prepare.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group',
    'test_guarded_prepare.ProbeRestorationChecks.test_setup_failure_restores_present_state',
    'test_guarded_prepare.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure',
    'test_guarded_prepare.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes',
    'test_guarded_prepare.ProbeRestorationChecks.test_trace_failure_archives_and_stops',
]
WORK = [
    'test_work_traces.PiChecks.test_real_accepted_and_rejected_candidates_and_zero',
    'test_work_traces.PiChecks.test_wrong_actual_second_division_rejected',
    'test_work_traces.MapChecks.test_all_real_tables_and_tail_windows',
    'test_work_traces.MapChecks.test_missing_overlapping_window_rejected',
    'test_work_traces.MapChecks.test_main_thread_substituted_for_map_owner_rejected',
    'test_work_traces.MapChecks.test_fixed_capacity_on_growth_fixture_rejected',
    'test_work_traces.TreeChecks.test_every_actual_root_and_retained_graph',
    'test_work_traces.TreeChecks.test_removed_real_leaf_rejected',
    'test_work_traces.TreeChecks.test_replaced_long_lived_root_rejected',
    'test_work_traces.TreeChecks.test_concurrently_live_churn_graph_cannot_overlap_retained_graph',
    'test_work_traces.TreeChecks.test_actual_worker_iteration_labels_must_match_each_interval',
    'test_work_traces.LoweringChecks.test_original_ir_leaf_path_must_allocate_unconditionally',
    'test_work_traces.LoweringChecks.test_final_arm_leaf_shortcut_before_allocation_rejected',
    'test_full_study.FailureIsolation.test_guard_admitted_runtime_output_or_resource_failure_can_continue_other_profiles',
    'test_full_study.FailureIsolation.test_frozen_guard_rejection_is_never_relabelled_source_failure',
    'test_full_study.FailureIsolation.test_every_ownership_environment_workload_or_pin_gate_is_literal_true',
    'test_full_study.FailureIsolation.test_exact_twenty_key_inventory_and_boolean_types_are_required',
]
OWNERSHIP += [
    'test_bootstrap_reuse.BootstrapReuse.test_exact_no_native_records_and_failed_first_build',
    'test_bootstrap_reuse.BootstrapReuse.test_partial_record_or_changed_failure_cannot_reuse_bootstrap',
    'test_bootstrap_reuse.BootstrapReuse.test_only_four_scored_strlen_pointer_casts_and_profiles_change',
]
OWNERSHIP += [
    'test_native_reuse.NativeReuse.test_exact_fifteen_profiles_126_records_preserved_without_relabelling',
    'test_native_reuse.NativeReuse.test_missing_changed_nonboolean_native_or_partial_profile_record_rejects',
    'test_native_reuse.NativeReuse.test_only_five_explicit_widening_casts_in_three_unqualified_forms',
]
INVENTORIES = {'ownership': OWNERSHIP, 'work': WORK}
MODULES = {'ownership': ['test_guarded_prepare','test_bootstrap_reuse','test_native_reuse'], 'work': ['test_work_traces', 'test_full_study']}


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
