"""Failure isolation rejects uncertain ownership, altered pins and bad environment."""
import copy
from types import SimpleNamespace
import unittest

import full_study


SAFE_GATES = ('sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
        'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
        'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
        'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
        'actual_full_workload', 'exact_command_and_group', 'pins_unchanged')


def retained_failure():
    checks = {key: True for key in SAFE_GATES}
    checks.update(native_exited_normally=False, no_execution_error=False, no_resource_violation=True,
                  complete_output_reference_match=False, complete_output_bytes_and_hash=False)
    return dict(error='Benchmark exited with status 1', error_type='RuntimeError',
        qualification_checks=checks)


class FailureIsolation(unittest.TestCase):
    def test_guard_admitted_runtime_output_or_resource_failure_can_continue_other_profiles(self):
        guard = SimpleNamespace(benchmark_failure=lambda attempt: True)
        result = retained_failure()
        original = copy.deepcopy(result)
        self.assertTrue(full_study.isolated_profile_failure(guard, result))
        self.assertEqual(result, original)

    def test_frozen_guard_rejection_is_never_relabelled_source_failure(self):
        guard = SimpleNamespace(benchmark_failure=lambda attempt: False)
        self.assertFalse(full_study.isolated_profile_failure(guard, retained_failure()))

    def test_every_ownership_environment_workload_or_pin_gate_is_literal_true(self):
        guard = SimpleNamespace(benchmark_failure=lambda attempt: True)
        for gate in SAFE_GATES:
            for value in (False, None, 1, 'true'):
                attempt = retained_failure()
                attempt['qualification_checks'][gate] = value
                with self.subTest(gate=gate, value=value):
                    self.assertFalse(full_study.isolated_profile_failure(guard, attempt))

    def test_exact_twenty_key_inventory_and_boolean_types_are_required(self):
        guard = SimpleNamespace(benchmark_failure=lambda attempt: True)
        missing = retained_failure()
        missing['qualification_checks'].pop('complete_output_bytes_and_hash')
        self.assertFalse(full_study.isolated_profile_failure(guard, missing))
        unknown = retained_failure()
        unknown['qualification_checks']['unreviewed-gate'] = True
        self.assertFalse(full_study.isolated_profile_failure(guard, unknown))
        for gate in retained_failure()['qualification_checks']:
            for value in (None, 0, 1, 'true'):
                malformed = retained_failure()
                malformed['qualification_checks'][gate] = value
                with self.subTest(gate=gate, value=value):
                    self.assertFalse(full_study.isolated_profile_failure(guard, malformed))
        non_dictionary = retained_failure()
        non_dictionary['qualification_checks'] = []
        self.assertFalse(full_study.isolated_profile_failure(guard, non_dictionary))


if __name__ == '__main__':
    unittest.main(verbosity=2)
