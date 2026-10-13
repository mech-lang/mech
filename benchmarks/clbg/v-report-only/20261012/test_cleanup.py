"""Failure-injection coverage for owned preparation processes and pin merges."""
import fcntl
from pathlib import Path
from types import SimpleNamespace
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import full_study
import prepare
import prepare_cohort

HERE = Path(__file__).resolve().parent


class Process:
    def __init__(self):
        self.pid = 12345
        self.returncode = None
        self.signals = []
        self.waits = 0

    def communicate(self, timeout):
        self.returncode = 0
        return b'compiled\n', b''

    def wait(self, timeout=None):
        self.waits += 1
        if self.returncode is None:
            self.returncode = 0
        return self.returncode

    def poll(self):
        return self.returncode

    def send_signal(self, signum):
        self.signals.append(signum)
        self.returncode = 130


class CleanupChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.saved = []
        self.cleaned = []
        self.process = Process()
        self.module = SimpleNamespace(now=lambda: 'fixture-time',
                                      identity=lambda pid: dict(pid=pid),
                                      save=lambda path, value: self.saved.append((path, dict(value))))
        self.harness = SimpleNamespace(group_members=lambda pid: [],
                                       kill_group=lambda pid: self.cleaned.append(pid))

    def tearDown(self):
        self.temporary.cleanup()

    def test_conflicting_pins_are_rejected(self):
        with self.assertRaisesRegex(AssertionError, 'Conflicting prerequisite pin'):
            prepare.merge_pins({'source': 'first'}, {'source': 'second'})

    def test_identical_pins_are_preserved(self):
        self.assertEqual(prepare.merge_pins({'source': 'same'}, {'source': 'same', 'binary': 'other'}),
                         {'source': 'same', 'binary': 'other'})

    def test_compiler_identity_failure_still_cleans_known_group(self):
        self.module.identity = lambda pid: (_ for _ in ()).throw(RuntimeError('identity failure'))
        with patch.object(prepare.subprocess, 'Popen', return_value=self.process) as popen:
            with self.assertRaisesRegex(RuntimeError, 'identity failure'):
                prepare.command_build(self.module, self.harness, ['compiler'], {}, self.directory, 42)
        self.assertEqual(self.cleaned, [12345])
        self.assertGreater(self.process.waits, 0)
        self.assertEqual(popen.call_args.kwargs['pass_fds'], (42,))
        self.assertTrue(popen.call_args.kwargs['start_new_session'])
        self.assertEqual(self.saved[-1][1]['pid'], 12345)
        self.assertEqual(self.saved[-1][1]['error'], 'identity failure')

    def test_compiler_timeout_still_cleans_known_group(self):
        self.process.communicate = lambda timeout: (_ for _ in ()).throw(subprocess.TimeoutExpired('compiler', timeout))
        with patch.object(prepare.subprocess, 'Popen', return_value=self.process):
            with self.assertRaises(subprocess.TimeoutExpired):
                prepare.command_build(self.module, self.harness, ['compiler'], {}, self.directory, 42)
        self.assertEqual(self.cleaned, [12345])
        self.assertGreater(self.process.waits, 0)

    def test_compiler_leftover_group_stops_build(self):
        self.harness.group_members = lambda pid: [dict(pid=12346)]
        with patch.object(prepare.subprocess, 'Popen', return_value=self.process):
            with self.assertRaisesRegex(RuntimeError, 'live child process'):
                prepare.command_build(self.module, self.harness, ['compiler'], {}, self.directory, 42)
        self.assertEqual(self.cleaned, [12345])
        self.assertGreater(self.process.waits, 0)

    def test_cohort_identity_failure_signals_only_owned_controller(self):
        self.module.identity = lambda pid: (_ for _ in ()).throw(RuntimeError('identity failure'))
        report = dict(active=None)
        with patch.object(prepare_cohort.subprocess, 'Popen', return_value=self.process):
            with self.assertRaisesRegex(RuntimeError, 'identity failure'):
                prepare_cohort.owned_preparation(self.module, ['prepare'], None, report,
                                                 self.directory / 'cohort.json', dict(benchmark='nbody'))
        self.assertEqual(self.process.signals, [prepare_cohort.signal.SIGINT])
        self.assertIsNone(report['active'])
        self.assertTrue(report['child_receipts'][0]['cleanup_confirmed'])
        self.assertEqual(report['child_receipts'][0]['pid'], 12345)

    def test_cohort_status_save_failure_signals_owned_controller(self):
        calls = []

        def save(path, value):
            calls.append(path)
            if len(calls) == 1:
                raise RuntimeError('journal failure')

        self.module.save = save
        report = dict(active=None)
        with patch.object(prepare_cohort.subprocess, 'Popen', return_value=self.process):
            with self.assertRaisesRegex(RuntimeError, 'journal failure'):
                prepare_cohort.owned_preparation(self.module, ['prepare'], None, report,
                                                 self.directory / 'cohort.json', dict(benchmark='nbody'))
        self.assertEqual(self.process.signals, [prepare_cohort.signal.SIGINT])
        self.assertIsNone(report['active'])
        self.assertTrue(report['child_receipts'][0]['cleanup_confirmed'])

    def test_completed_controller_needs_no_signal(self):
        report = dict(active=None)
        with patch.object(prepare_cohort.subprocess, 'Popen', return_value=self.process):
            status = prepare_cohort.owned_preparation(self.module, ['prepare'], None, report,
                                                      self.directory / 'cohort.json', dict(benchmark='nbody'))
        self.assertEqual(status, 0)
        self.assertEqual(self.process.signals, [])
        self.assertIsNone(report['active'])
        self.assertTrue(report['child_receipts'][0]['cleanup_confirmed'])


def main():
    module = full_study.configure('nbody')
    print('WAIT LOCK: V process cleanup regression checks', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(CleanupChecks)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        destination = HERE / 'cleanup-fixtures.json'
        assert not destination.exists(), 'Never overwrite a retained fixture receipt'
        module.save(destination, dict(
            status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), created_at=module.now(),
            full_performance_runs=0, report_only=True, never_chart_eligible=True,
            shared_timing_lock=str(module.LOCK),
            pins={str(path): module.sha(path) for path in (
                Path(__file__).resolve(), HERE / 'prepare.py', HERE / 'prepare_cohort.py',
                HERE / 'full_study.py', full_study.HELPER,
            )},
        ))
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
