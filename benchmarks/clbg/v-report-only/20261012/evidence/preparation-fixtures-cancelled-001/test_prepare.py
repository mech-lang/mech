"""Bounded regression checks for temporary harness override ownership."""
import fcntl
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import full_study
import prepare

HERE = Path(__file__).resolve().parent


class ProbeOwnershipChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.calls = []
        self.execute = lambda *args: None
        self.validate = lambda *args: dict(reference_match=True)
        self.harness = SimpleNamespace(
            execute=self.execute, validate=self.validate,
            WORK=dict(nbody=(1000, 50000000), knucleotide=(1000, 25000000)),
            INPUTS=dict(knucleotide=(Path('/retained/small'), Path('/retained/full'))),
        )
        self.work = dict(self.harness.WORK)
        self.inputs = dict(self.harness.INPUTS)
        self.module = SimpleNamespace(
            OUT=self.directory, REPORT=self.directory / 'probes.json',
            save=lambda *args: None, one_attempt=self.one_attempt,
        )
        self.row = dict(id='v-test', benchmark='nbody')
        self.manifest = dict(size=1000, timeout_seconds=300, maximum_rss_mib=8192)
        self.report = dict(attempts=[], active=None)

    def tearDown(self):
        self.temporary.cleanup()

    def one_attempt(self, harness, row, manifest, report, round_index, lock_fd):
        self.calls.append((dict(harness.WORK), dict(harness.INPUTS), manifest['size']))
        self.assertIsNot(harness.execute, self.execute)
        self.assertIsNot(harness.validate, self.validate)
        return dict(qualified=True, full_workload=False,
                    supervised_native_group=dict(inherited_common_mutex=True),
                    validation=dict(reference_match=True))

    def assert_restored(self):
        self.assertIs(self.harness.execute, self.execute)
        self.assertIs(self.harness.validate, self.validate)
        self.assertEqual(self.harness.WORK, self.work)
        self.assertEqual(self.harness.INPUTS, self.inputs)

    def test_success_restores_numeric_override(self):
        prepare.probe(self.module, self.harness, self.row, 'canonical-small',
                      self.manifest, self.report, -1, n=7)
        self.assertEqual(self.calls[0][0]['nbody'], (7, 50000000))
        self.assertEqual(self.calls[0][2], 7)
        self.assert_restored()

    def test_oracle_failure_restores_before_any_sampler(self):
        with patch.object(prepare.references, 'nbody', side_effect=RuntimeError('injected oracle failure')):
            with self.assertRaisesRegex(RuntimeError, 'oracle failure'):
                prepare.probe(self.module, self.harness, self.row, 'oracle-error',
                              self.manifest, self.report, -1, n=7)
        self.assertEqual(self.calls, [])
        self.assert_restored()

    def test_input_oracle_failure_restores_input_pair(self):
        self.row['benchmark'] = 'knucleotide'
        with patch.object(prepare.references, 'knucleotide', side_effect=RuntimeError('injected input oracle failure')):
            with self.assertRaisesRegex(RuntimeError, 'input oracle failure'):
                prepare.probe(self.module, self.harness, self.row, 'input-error',
                              self.manifest, self.report, -1, data=b'>THREE\nACGT\n')
        self.assertEqual(self.calls, [])
        self.assert_restored()

    def test_sampler_failure_restores_both_functions(self):
        self.module.one_attempt = lambda *args: (_ for _ in ()).throw(RuntimeError('injected sampler failure'))
        with self.assertRaisesRegex(RuntimeError, 'sampler failure'):
            prepare.probe(self.module, self.harness, self.row, 'canonical-small',
                          self.manifest, self.report, -1)
        self.assert_restored()

    def test_unqualified_attempt_is_retained_and_stops(self):
        self.module.one_attempt = lambda *args: dict(qualified=False, error='injected output mismatch')
        with self.assertRaisesRegex(AssertionError, 'bounded probe failed'):
            prepare.probe(self.module, self.harness, self.row, 'canonical-small',
                          self.manifest, self.report, -1)
        self.assertEqual(len(self.report['attempts']), 1)
        self.assertFalse(self.report['attempts'][0]['qualified'])
        self.assertIsNone(self.report['active'])
        self.assert_restored()


def main():
    module = full_study.configure('nbody')
    print('WAIT LOCK: V preparation override regression checks', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('LOCKED: V preparation override regression checks', flush=True)
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ProbeOwnershipChecks)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        destination = HERE / 'preparation-fixtures.json'
        assert not destination.exists(), 'Never overwrite a retained fixture receipt'
        receipt = dict(
            status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors),
            created_at=module.now(), full_performance_runs=0,
            report_only=True, never_chart_eligible=True,
            shared_timing_lock=str(module.LOCK),
            pins={str(path): module.sha(path) for path in (
                Path(__file__).resolve(), HERE / 'prepare.py', HERE / 'references.py',
                HERE / 'full_study.py', full_study.HELPER,
            )},
        )
        module.save(destination, receipt)
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
