"""Unexecuted forged observation fixtures; no runtime proof is fabricated."""
import copy
import unittest
import proof_candidate as proof


def observation():
    return dict(actual_common_mutex_held=True, actual_native_process_owned=True,
        actual_compile_disabled=True, same_unchanged_scored_binary_or_unscored_canary=True,
        no_observer_drop_or_detach=True, all_thread_events_observed=True,
        observer_armed_before_runtime_initialization=True, observer_retained_through_shutdown=True,
        missing_native_code_error=False, exit_status=0, trace_compile_bytes=0,
        hook_evidence={symbol: dict(symbol=symbol, resolved_address='synthetic-address',
            actual_resolved_image_sha256='synthetic-pin', raw_resolution_artifact='synthetic-resolution',
            raw_events_artifact='synthetic-events', actual_linkage_name='synthetic-linkage',
            reviewed_source_route='synthetic-source-route', entry_count=0)
            for symbol in proof.INTERPRETER_HOOKS | proof.JIT_HOOKS},
        unscored_negative_canary=True, actual_target_native_coverage_absent=True,
        actual_target_method_identity_artifact='synthetic-target', expected_input_not_frozen=True)


class ProofCandidate(unittest.TestCase):
    def test_empty_trace_with_interpreter_entry_is_rejected(self):
        for symbol in proof.INTERPRETER_HOOKS:
            record = observation()
            record['hook_evidence'][symbol]['entry_count'] = 1
            self.assertEqual(proof.require_actual_negative_canary(record)['status'],
                             'negative-canary-rejected-by-observation-contract')

    def test_zero_events_do_not_validate_missing_native_canary(self):
        with self.assertRaisesRegex(AssertionError, 'canary was accepted'):
            proof.require_actual_negative_canary(observation())

    def test_missing_code_or_native_jit_rejected_without_trace_dependency(self):
        missing = observation(); missing['missing_native_code_error'] = True
        self.assertIn('missing-code', proof.require_actual_negative_canary(missing)['rejection'])
        for symbol in proof.JIT_HOOKS:
            jit = observation(); jit['hook_evidence'][symbol]['entry_count'] = 1
            self.assertIn('code generation', proof.require_actual_negative_canary(jit)['rejection'])

    def test_unarmed_dropped_or_incomplete_hooks_cannot_qualify(self):
        for key in ('observer_armed_before_runtime_initialization', 'all_thread_events_observed',
                    'no_observer_drop_or_detach', 'observer_retained_through_shutdown'):
            record = observation(); record[key] = False
            with self.assertRaises(AssertionError): proof.inspect_observation(record)
        record = observation(); record['hook_evidence'].pop('jl_interpret_opaque_closure')
        with self.assertRaises(AssertionError): proof.inspect_observation(record)

    def test_zero_event_record_remains_unadmitted_without_raw_review(self):
        result = proof.inspect_observation(observation())
        self.assertIs(result['runtime_aot_proven'], False)
        self.assertIs(result['chart_admission'], False)


if __name__ == '__main__': unittest.main()
