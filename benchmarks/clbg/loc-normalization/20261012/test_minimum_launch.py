"""Mocked ownership failures; never starts or signals a real child."""
import fcntl
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch

import pending_minima_study as study

HERE = Path(__file__).resolve().parent


class LaunchChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.saved = []
        self.harness = SimpleNamespace(kill_group=Mock(),group_members=Mock(return_value=[]))
        self.child = Mock(pid=12345, returncode=-9)
        self.child.poll.return_value = None
        self.module = SimpleNamespace(OUT=Path(self.temporary.name),
            now=lambda:'fixture-time', identity=Mock(return_value={'pid':12345}),
            load_harness=lambda:self.harness, save=self.save)

    def save(self, path, value):
        self.saved.append((path.name, dict(value)))

    def call(self):
        with patch.object(study.subprocess, 'Popen', return_value=self.child):
            return study.launch_controller(self.module, ['fixture-command'], 'f'*64, 'pending')

    def test_success_transfers_only_after_durable_receipt(self):
        receipt = self.call()
        self.assertEqual(self.saved[0][0], 'launch.json')
        self.assertEqual(receipt['pid'],12345)
        self.harness.kill_group.assert_not_called()
        self.child.wait.assert_not_called()

    def test_identity_failure_cleans_owned_controller(self):
        self.module.identity.side_effect = RuntimeError('identity failure')
        with self.assertRaisesRegex(RuntimeError,'identity failure'):
            self.call()
        self.harness.kill_group.assert_called_once_with(12345)
        self.child.wait.assert_called_once()
        self.assertEqual(self.saved[-1][0],'launch-failure.json')
        self.assertFalse(self.saved[-1][1]['ownership_transferred'])

    def test_journal_failure_cleans_before_releasing_ownership(self):
        def fail_launch(path,value):
            if path.name == 'launch.json':
                raise OSError('journal failure')
            self.save(path,value)
        self.module.save = fail_launch
        with self.assertRaisesRegex(OSError,'journal failure'):
            self.call()
        self.harness.kill_group.assert_called_once_with(12345)
        self.child.wait.assert_called_once()
        self.assertTrue(self.saved[-1][1]['cleanup_confirmed'])

    def test_cleanup_error_still_waits_and_preserves_failure(self):
        self.module.identity.side_effect = RuntimeError('identity failure')
        self.harness.kill_group.side_effect = RuntimeError('cleanup fixture failure')
        with self.assertRaisesRegex(RuntimeError,'cleanup failed'):
            self.call()
        self.child.wait.assert_called_once()
        self.child.kill.assert_called_once()
        self.assertEqual(self.saved[-1][0],'launch-failure.json')
        self.assertTrue(self.saved[-1][1]['cleanup_confirmed'])
        self.assertTrue(self.saved[-1][1]['fallback_owned_controller_kill'])
        self.assertIn('cleanup fixture failure',self.saved[-1][1]['cleanup_error'])


def main():
    module = study.configure('spectralnorm')
    print('WAIT common mutex: compact minimum controller ownership fixtures',flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        destination=HERE/'minimum-launch-fixtures.json'
        assert not destination.exists(),'Never overwrite a fixture receipt'
        files=[Path(__file__).resolve(),HERE/'pending_minima_study.py',
               study.VROOT/'prepare.py',study.VROOT/'full_study.py',study.v_sampler.HELPER]
        pins={str(path):module.sha(path) for path in files}
        result=unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(LaunchChecks))
        module.load_harness().PinGuard(pins).verify()
        module.save(destination,dict(status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
            created_at=module.now(),pins=pins,pins_unchanged_at_end=True,
            actual_native_runs=0,chart_admitted=False))
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
