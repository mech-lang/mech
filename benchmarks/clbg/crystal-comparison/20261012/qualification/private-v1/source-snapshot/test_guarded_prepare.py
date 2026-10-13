"""Twelve authored unexecuted preparation ownership/restoration fixtures.

Only mocked children/signals. Standalone execution requires the common mutex;
the resulting proof has no native benchmark or compiler processes.
"""
import fcntl
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import guarded_prepare as preparation

FD = None


class PreparationChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the locked standalone fixture entry point')
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.original_validator = Mock()
        self.harness = SimpleNamespace(WORK={'pidigits': (27, 10000)},
            INPUTS={'pidigits': ('old-small', 'old-full')}, validate=self.original_validator,
            group_members=Mock(return_value=[]), kill_group=Mock())
        self.guard = SimpleNamespace(now=lambda: 'mock-time', identity=Mock(return_value={'pid': 123}),
            save=Mock(), REPORT=self.directory / 'report.json', OUT=self.directory,
            one_attempt=Mock(return_value=dict(qualified=True, full_workload=False, attempt=1)))
        self.row = dict(id='mock-row', benchmark='pidigits')
        self.report = dict(attempts=[], active=None)

    def probe(self, data=None):
        return preparation.probe(self.guard, self.harness, self.row, 2, data, b'31        \t:2\n', {},
            self.directory / 'output', FD, self.report, 'mock-case')

    def assert_restored(self):
        self.assertEqual(self.harness.WORK, {'pidigits': (27, 10000)})
        self.assertIs(self.harness.validate, self.original_validator)

    def test_success_restores_existing_work_input_validator(self):
        old = self.harness.INPUTS['pidigits']
        self.probe(b'input')
        self.assert_restored()
        self.assertIs(self.harness.INPUTS['pidigits'], old)
        self.assertEqual(len(self.report['attempts']), 1)

    def test_absent_input_restored_as_absent(self):
        self.harness.INPUTS.clear()
        self.probe(b'input')
        self.assert_restored()
        self.assertNotIn('pidigits', self.harness.INPUTS)

    def test_input_write_error_restores_scope_before_launch(self):
        original_write = Path.write_bytes

        def write(path, value):
            if path.name == 'stdin.fasta':
                raise OSError('mock input write failure')
            return original_write(path, value)

        with patch.object(Path, 'write_bytes', write), self.assertRaises(OSError):
            self.probe(b'input')
        self.assert_restored()
        self.guard.one_attempt.assert_not_called()

    def test_expected_write_error_does_not_launch(self):
        with (patch.object(Path, 'write_bytes', side_effect=OSError('mock oracle write failure')),
              self.assertRaises(OSError)):
            self.probe()
        self.assert_restored()
        self.guard.one_attempt.assert_not_called()

    def test_sampler_error_restores_scope(self):
        raw = self.directory / 'attempts/001-mock-row.json'
        raw.parent.mkdir()
        original = b'{"seconds":0.25,"exit_status":0,"error":"actual sampler error"}'
        raw.write_bytes(original)
        self.guard.one_attempt.side_effect = RuntimeError('mock late supervisor failure')
        with self.assertRaises(RuntimeError):
            self.probe(b'input')
        self.assert_restored()
        self.assertEqual(self.harness.INPUTS['pidigits'], ('old-small', 'old-full'))
        self.assertFalse(self.report['attempts'][0]['qualified'])
        self.assertEqual(self.report['attempts'][0]['seconds'], 0.25)
        self.assertEqual(raw.read_bytes(), original)
        self.assertEqual(self.report['attempts'][0]['execution_failure_before_preparation_error']['error'], 'actual sampler error')

    def test_unqualified_attempt_retained_and_scope_restored(self):
        self.guard.one_attempt.return_value = dict(qualified=False, full_workload=False, error='mock integrity')
        with self.assertRaises(AssertionError):
            self.probe()
        self.assertEqual(self.report['attempts'][0]['error'], 'mock integrity')
        self.assert_restored()
        self.guard.one_attempt.return_value = dict(qualified=True, full_workload=False, attempt=2)
        with self.assertRaises(AssertionError):
            preparation.probe(self.guard, self.harness, self.row, 2, None, b'expected', {},
                self.directory / 'output', FD, self.report, 'trace-error',
                trace=lambda directory: (_ for _ in ()).throw(AssertionError('mock work mismatch')))
        self.assertFalse(self.report['attempts'][1]['qualified'])
        self.assertTrue(self.report['attempts'][1]['output_qualified_before_source_work'])
        self.assertTrue((self.directory / 'output/mock-row/trace-error/failed-work-review.json').is_file())
        self.assert_restored()

    def command(self, process=None):
        process = process or SimpleNamespace(pid=123, returncode=0,
            stdout=None, stderr=None, poll=lambda: None,
            communicate=Mock(return_value=(b'complete', b'')), wait=Mock(return_value=0))
        with patch.object(preparation.subprocess, 'Popen', return_value=process) as popen:
            value = preparation.owned_command(self.guard, self.harness, ['mock-compiler'], {},
                self.directory / 'command', FD)
        return value, process, popen

    def test_command_inherits_lock_and_owns_new_session(self):
        value, process, popen = self.command()
        self.assertEqual(value['stdout'], b'complete')
        self.assertTrue(popen.call_args.kwargs['start_new_session'])
        self.assertEqual(popen.call_args.kwargs['pass_fds'], (FD,))
        self.harness.kill_group.assert_called_once_with(123)
        process.wait.assert_called_once()

    def test_popen_failure_has_no_foreign_cleanup(self):
        with (patch.object(preparation.subprocess, 'Popen', side_effect=OSError('mock spawn error')),
              self.assertRaises(OSError)):
            preparation.owned_command(self.guard, self.harness, ['mock-compiler'], {},
                                      self.directory / 'command', FD)
        self.harness.kill_group.assert_not_called()
        self.assertTrue((self.directory / 'command/command.json').is_file())

    def test_identity_failure_still_reaps_owned_child(self):
        self.guard.identity.side_effect = RuntimeError('mock identity error')
        with self.assertRaises(RuntimeError):
            self.command()
        self.harness.kill_group.assert_called_once_with(123)

    def test_active_journal_failure_still_reaps_owned_child(self):
        self.guard.save.side_effect = OSError('mock journal error')
        with self.assertRaises(OSError):
            self.command()
        self.harness.kill_group.assert_called_once_with(123)

    def test_late_child_group_rejects_and_cleans(self):
        self.harness.group_members.return_value = [{'pid': 124}]
        with self.assertRaises(AssertionError):
            self.command()
        self.harness.kill_group.assert_called_once_with(123)
        detail = json.loads((self.directory / 'command/command.json').read_text())
        self.assertIn('error', detail)

    def test_conflicting_pin_rejected(self):
        self.assertEqual(preparation.merge({'x': 'a'}, {'x': 'a'}), {'x': 'a'})
        with self.assertRaises(AssertionError):
            preparation.merge({'x': 'a'}, {'x': 'b'})


def main():
    global FD
    print('WAIT LOCK: Crystal mocked preparation fixtures', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        FD = lock.fileno()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(PreparationChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = preparation.HERE / 'preparation-fixtures.log'
        assert not log.exists(), 'Never overwrite a fixture attempt'
        with log.open('x') as stream:
            stream.write(text)
        receipt = dict(status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), native_probes=0,
            actual_signals_sent=0, never_chart_eligible=True, shared_timing_lock=str(preparation.LOCK),
            pins={str(p): preparation.sha(p) for p in (Path(__file__).resolve(),
                preparation.HERE / 'guarded_prepare.py', log)})
        preparation.new_json(preparation.HERE / 'preparation-fixtures.json', receipt)
        if not result.wasSuccessful() or result.testsRun != 12:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
