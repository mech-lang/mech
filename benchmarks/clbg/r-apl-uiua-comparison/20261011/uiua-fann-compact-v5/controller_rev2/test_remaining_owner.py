"""Mocked continuation ledger/oracle/gate fixtures; no process or signal probe."""
from copy import deepcopy
from contextlib import redirect_stdout
import fcntl
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import random
from types import SimpleNamespace
import unittest

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')


def load():
    spec = importlib.util.spec_from_file_location('isolated_remaining_owner_fixture', HERE / 'remaining_owner.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def copy_aliases(report):
    for row in report['programs'].values():
        target_key = row.get('timing_alias_of')
        if target_key:
            target = report['programs'][target_key]
            for key in ('samples_seconds', 'sample_validation', 'median_seconds', 'sample_stdev_seconds', 'full_validation', 'full_failure', 'measurement_status'):
                if key in target:
                    row[key] = deepcopy(target[key])


class Resume(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.harness = SimpleNamespace(copy_aliases=copy_aliases, compact=SimpleNamespace(HASHES={'mandelbrot':[(1, 'small'), (20, 'full-pbm')]}))
        self.row = dict(id='source', key='source', benchmark='fannkuchredux', language='r', command=['runtime', 'program', '{n}'],
            samples_seconds=[100.0], sample_validation=[dict(reference_match=True)], attempts=[1], measurement_status='partial')
        self.old_attempt = dict(key='source', sample=1, seconds=100.0, qualified=True, validation=dict(reference_match=True))
        self.report = dict(programs={'source':self.row, 'alias':dict(self.row, key='alias', timing_alias_of='source')},
            attempts=[deepcopy(self.old_attempt)], orders={}, completed_benchmarks=[], active=dict(key='source', sample=2))
        self.manifest = dict(size=12, canonical_full_workload=True, full_output_reference=dict(bytes=28, sha256='complete-hash'))
        self.native = dict(command=['runtime', 'program', '12'], pid=1002, pgid=1002, inherited_common_mutex=True)
        self.result = dict(exit_status=0, command=deepcopy(self.native['command']), full_workload=True, workload_n=12,
            stdout_bytes=28, stdout_sha256='complete-hash', validation=dict(reference_match=True), valid_environment=True,
            pins_unchanged_at_end=True, sampler_wait_status=0, supervised_native_group=deepcopy(self.native),
            qualified=True, seconds=90.0)

    def test_remaining_stages_exclude_uiua_and_alias_launches(self):
        self.assertEqual(self.m.STAGES, [('r','fannkuchredux'), ('apl','mandelbrot'), ('apl','fasta'), ('apl','knucleotide'), ('apl','binarytrees'), ('apl','nbody'), ('apl','fannkuchredux')])
        rows = self.m.ordered_candidates(self.report, 'r', 'fannkuchredux', 0)
        self.assertEqual([row['key'] for row in rows], ['source'])
        with self.assertRaises(AssertionError):
            self.m.ordered_candidates(self.report, 'uiua', 'fannkuchredux', 0)

    def test_existing_round_order_must_match(self):
        self.report['programs']['second'] = dict(self.row, id='second', key='second')
        expected = [self.row, self.report['programs']['second']]
        random.Random(20261011 + sum(map(ord, 'r'+'fannkuchredux'))).shuffle(expected)
        self.report['orders']['fannkuchredux'] = [[row['key'] for row in expected]]
        self.assertEqual([row['key'] for row in self.m.ordered_candidates(self.report, 'r', 'fannkuchredux', 0)], self.report['orders']['fannkuchredux'][0])
        self.report['orders']['fannkuchredux'][0].reverse()
        with self.assertRaises(AssertionError):
            self.m.ordered_candidates(self.report, 'r', 'fannkuchredux', 0)

    def test_qualified_resume_preserves_old_sample_and_attempt(self):
        old = deepcopy(self.report['attempts'])
        self.assertTrue(self.m.record_attempt(self.harness, self.report, self.row, deepcopy(self.result), 2, 'reviewed-manifest'))
        self.assertEqual(self.report['attempts'][:1], old)
        self.assertEqual(self.row['samples_seconds'], [100.0, 90.0])
        self.assertEqual(self.row['attempts'], [1, 2])
        self.assertEqual(self.report['attempts'][-1]['sample'], 2)
        self.assertEqual(self.report['programs']['alias']['samples_seconds'], [100.0, 90.0])
        self.assertNotIn('active', self.report)

    def test_all_fatal_qualification_gates_reject(self):
        self.assertTrue(all(self.m.qualification_checks(self.result, self.native, self.manifest, self.row, 0).values()))
        mutations = {'exit_status':1, 'error':'bad', 'sampler_failure':True, 'registration_error':'bad', 'supervisor_pin_failure':True,
            'resource_violation':{'limit':1}, 'resource_monitor_errors':['bad'], 'monitor_errors':['bad'],
            'monitor_finish_error':'bad', 'cleanup_error':'bad', 'remaining_group_members':[1],
            'supervisor_cleanup':True, 'supervisor_remaining_members':[1], 'valid_environment':False,
            'full_workload':False, 'workload_n':11, 'command':['different'], 'stdout_bytes':27,
            'stdout_sha256':'different', 'validation':dict(reference_match=False), 'pins_unchanged_at_end':False}
        for key, value in mutations.items():
            with self.subTest(key=key):
                result = dict(self.result, **{key:value})
                self.assertFalse(all(self.m.qualification_checks(result, self.native, self.manifest, self.row, 0).values()))
        self.assertFalse(all(self.m.qualification_checks(self.result, self.native, self.manifest, self.row, 256).values()))
        self.assertFalse(all(self.m.qualification_checks(self.result, None, self.manifest, self.row, 0).values()))
        self.assertFalse(all(self.m.qualification_checks(self.result, dict(self.native,inherited_common_mutex=False), self.manifest, self.row, 0).values()))

    def test_integrity_failure_is_retained_without_source_failure(self):
        attempt = dict(self.result, error='raw sampler failure', error_type='RuntimeError', sampler_failure=True)
        def pin_failure(pins):
            raise AssertionError('pin failed after execute')
        self.m.verify = pin_failure
        self.assertFalse(self.m.verify_attempt_pins(attempt, {}))
        self.assertEqual(attempt['execution_failure_before_supervisor_error'],dict(error='raw sampler failure',error_type='RuntimeError'))
        attempt['qualified'] = all(self.m.qualification_checks(attempt,self.native,self.manifest,self.row,0).values())
        self.assertFalse(self.m.record_attempt(self.harness, self.report, self.row, attempt, 2, 'reviewed-manifest'))
        self.assertEqual(self.row['samples_seconds'], [100.0])
        self.assertNotIn('full_failure', self.row)
        self.assertEqual(self.report['attempts'][-1]['error'], 'pin failed after execute')
        self.assertEqual(self.row['measurement_status'], 'continuation-stopped-unqualified')

    def test_real_own_timeout_is_retained_without_a_sample(self):
        attempt = dict(self.result, qualified=False, error='recorded own timeout', error_type='TimeoutError', exit_status=-9)
        self.assertFalse(self.m.record_attempt(self.harness, self.report, self.row, attempt, 2, 'reviewed-manifest'))
        self.assertEqual(self.row['samples_seconds'], [100.0])
        self.assertEqual(self.row['full_failure']['error_type'], 'TimeoutError')
        self.assertEqual(self.report['attempts'][-1]['attempt_number'], 2)
        self.assertEqual(self.report['programs']['alias']['full_failure'], self.row['full_failure'])

    def test_complete_oracles_preserve_nbody_numeric_validation(self):
        self.assertIsNone(self.m.full_reference(self.harness, 'nbody'))
        data = b'3968050\nPfannkuchen(12) = 65\n'
        self.assertEqual(self.m.full_reference(self.harness, 'fannkuchredux'), dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
        self.assertEqual(self.m.full_reference(self.harness, 'mandelbrot'),dict(bytes=20,sha256='full-pbm'))
        with self.assertRaises(AssertionError):
            self.m.full_reference(self.harness, 'unknown')

    def test_completion_metadata_preserves_older_failures(self):
        self.row['samples_seconds'] = [100.0,90.0,80.0]
        self.report['programs']['alias']['samples_seconds'] = [100.0,90.0,80.0]
        failure = dict(error='retained genuine legacy error')
        self.report['programs']['old-failed'] = dict(key='old-failed',benchmark='revcomp',samples_seconds=[],full_failure=deepcopy(failure))
        self.m.refresh_report_status(self.report)
        self.assertEqual(self.report['programs']['old-failed']['full_failure'],failure)
        self.assertEqual(set(self.report['completed_benchmarks']), {'fannkuchredux','revcomp'})
        self.assertEqual(self.report['status'],'complete-with-failures')
        self.assertEqual(self.report['attempts'],[self.old_attempt])


def main():
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        receipt = HERE / 'remaining-fixtures.json'
        assert not receipt.exists(), 'Do not overwrite or rerun sealed continuation fixtures'
        log=io.StringIO()
        with redirect_stdout(log):
            result=unittest.TextTestRunner(stream=log,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Resume))
        module=load()
        path=HERE/'remaining-fixtures.log'
        path.write_text(log.getvalue())
        receipt.write_text(json.dumps(dict(status='passed' if result.wasSuccessful() else 'failed',cases=result.testsRun,
            failures=len(result.failures),errors=len(result.errors),runner_sha256=module.sha(HERE/'remaining_owner.py'),
            fixture_source_sha256=module.sha(__file__),log_sha256=module.sha(path),actual_signals_sent=0,native_probes=0,
            shared_mutex=str(LOCK),mocked_attempts_and_harness=True),indent=2)+'\n')
        print(log.getvalue(),flush=True)
        assert result.wasSuccessful() and result.testsRun==8


if __name__=='__main__':
    main()
