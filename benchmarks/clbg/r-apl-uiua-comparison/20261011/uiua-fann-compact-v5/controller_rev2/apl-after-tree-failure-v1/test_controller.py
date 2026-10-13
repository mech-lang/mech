"""Meaningful recovery boundary corruption checks; no native subject or signal."""
import copy
from types import SimpleNamespace
import unittest
import controller


def failure():
    checks = {key: True for key in controller.GATES}
    checks.update(no_execution_error=False, complete_output_reference_match=False, complete_output_bytes_and_hash=False)
    return dict(qualification_checks=checks, sampler_wait_status=0, qualified=False,
        id='binarytrees-apl-compact', attempt=75, workload_n=21, full_workload=True,
        error_type='OutputValidationError', stdout_bytes=0, exit_status=0,
        stderr='WS FULL+\nMake[4]  Left←Make D-1\n')


def evidence():
    bad = failure()
    rows = {}
    for benchmark in ('nbody', 'fannkuchredux'):
        for name, language in (('compact', 'apl'), ('optimized', 'apl'), ('control', 'mech')):
            key = benchmark + ':' + name
            row = dict(benchmark=benchmark, language=language, samples_seconds=[], sample_validation=[])
            if name == 'optimized': row['timing_alias_of'] = benchmark + ':compact'
            rows[key] = row
    old = dict(status='partial', attempts=[{} for _ in range(74)] + [bad], programs=rows)
    state = dict(status='stopped', supervisor=dict(pid=17798), manifest_sha256=controller.PRIOR_MANIFEST_SHA,
        error_type='RuntimeError', error='Remaining actual sample failed qualification; evidence retained and no retry',
        stages=[dict(benchmark='fasta', status='complete'), dict(benchmark='knucleotide', status='complete'), dict(benchmark='binarytrees')])
    return old, state, bad


class RecoveryChecks(unittest.TestCase):
    def test_exact_typed_failure_isolation_rejects_each_safety_and_inventory_corruption(self):
        guard = SimpleNamespace(benchmark_failure=lambda value: True)
        self.assertTrue(controller.isolated(guard, failure()))
        for key in controller.SAFE:
            for value in (False, None, 1, 'true'):
                item = failure(); item['qualification_checks'][key] = value
                with self.subTest(key=key, value=value): self.assertFalse(controller.isolated(guard, item))
        for change in ('missing', 'extra', 'bool-wait', 'qualified', 'guard-rejected'):
            item = failure(); selected_guard = guard
            if change == 'missing': item['qualification_checks'].pop('complete_output_reference_match')
            elif change == 'extra': item['qualification_checks']['invented'] = True
            elif change == 'bool-wait': item['sampler_wait_status'] = False
            elif change == 'qualified': item['qualified'] = True
            else: selected_guard = SimpleNamespace(benchmark_failure=lambda value: False)
            with self.subTest(change=change): self.assertFalse(controller.isolated(selected_guard, item))

    def test_failed_tree_and_all75_original_records_remain_unchanged_in_additive_seed(self):
        old, state, bad = evidence(); original = copy.deepcopy(old)
        result = controller.seed(old, state, bad, SimpleNamespace(benchmark_failure=lambda value: True))
        self.assertEqual(old, original)
        self.assertEqual(result['attempts'], old['attempts'])
        self.assertEqual(result['programs'], old['programs'])
        self.assertEqual(result['independent_recovery']['full_processes_planned'], 12)
        self.assertTrue(result['independent_recovery']['tree_control_pending'])
        for change in ('missing-original-record', 'wrong-stage', 'wrong-tree-n', 'missing-real-failure'):
            altered, terminal, actual = evidence()
            if change == 'missing-original-record': altered['attempts'].pop(0)
            elif change == 'wrong-stage': terminal['stages'][1]['benchmark'] = 'nbody'
            elif change == 'wrong-tree-n': actual['workload_n'] = 7
            else: actual['stderr'] = ''
            with self.subTest(change=change), self.assertRaises(AssertionError):
                controller.seed(altered, terminal, actual, SimpleNamespace(benchmark_failure=lambda value: True))

    def test_already_measured_or_failed_selected_source_and_alias_duplication_cannot_recover(self):
        for change in ('sample', 'failure', 'alias-removed', 'extra-executor'):
            old, state, bad = evidence()
            if change == 'sample': old['programs']['nbody:compact']['samples_seconds'] = [1.0]
            elif change == 'failure': old['programs']['nbody:compact']['full_failure'] = {'actual': True}
            elif change == 'alias-removed': old['programs']['fannkuchredux:optimized'].pop('timing_alias_of')
            else: old['programs']['nbody:other'] = dict(benchmark='nbody', language='apl', samples_seconds=[], sample_validation=[])
            with self.subTest(change=change), self.assertRaises(AssertionError):
                controller.seed(old, state, bad, SimpleNamespace(benchmark_failure=lambda value: True))
