"""Mocked fatal/isolated source gate fixtures; execution holds common mutex."""
import copy
from types import SimpleNamespace
import unittest
import full_study


class IsolatedFailures(unittest.TestCase):
    def setUp(self):
        self.guard = SimpleNamespace(benchmark_failure=lambda attempt: bool(attempt.get('source_failure')))
        self.safe = ('sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
                     'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
                     'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
                     'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
                     'actual_full_workload', 'exact_command_and_group', 'pins_unchanged')
        self.attempt = dict(source_failure=True, qualification_checks=dict.fromkeys(self.safe, True))

    def test_real_source_failure_with_literal_safe_gates_is_isolated(self):
        self.assertTrue(full_study.isolated_profile_failure(self.guard, self.attempt))

    def test_every_unsafe_gate_stops_and_truthy_values_do_not_waive_it(self):
        for key in self.safe:
            for value in (False, None, 1, 'true'):
                attempt = copy.deepcopy(self.attempt); attempt['qualification_checks'][key] = value
                self.assertFalse(full_study.isolated_profile_failure(self.guard, attempt), (key, value))
            attempt = copy.deepcopy(self.attempt); del attempt['qualification_checks'][key]
            self.assertFalse(full_study.isolated_profile_failure(self.guard, attempt), key)

    def test_missing_gates_and_non_source_failure_stop(self):
        self.assertFalse(full_study.isolated_profile_failure(self.guard, dict(source_failure=True)))
        self.attempt['source_failure'] = False
        self.assertFalse(full_study.isolated_profile_failure(self.guard, self.attempt))

if __name__ == '__main__': unittest.main()
