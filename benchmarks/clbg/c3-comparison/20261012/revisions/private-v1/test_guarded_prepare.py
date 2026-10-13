"""Mocked ownership/restoration fixtures; no native process or signal is used."""
import io
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import guarded_prepare as preparation


class FakeProcess:
    pid = 123456

    def __init__(self, timeout=False):
        self.returncode = None
        self.stdout = io.BytesIO()
        self.stderr = io.BytesIO()
        self.timeout = timeout
        self.calls = 0
        self.waited = False

    def poll(self):
        return self.returncode

    def communicate(self, timeout):
        self.calls += 1
        if self.timeout and self.calls == 1:
            raise subprocess.TimeoutExpired(['fixture'], timeout, output=b'partial', stderr=b'warning')
        self.returncode = 0
        return b'complete', b'complete-stderr'

    def wait(self, timeout):
        assert timeout is not None and timeout > 0
        self.waited = True
        self.returncode = self.returncode if self.returncode is not None else -15
        return self.returncode


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value))


class OwnershipChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.output = Path(self.temporary.name)
        self.cleaned = []
        self.guard = SimpleNamespace(now=lambda: 'fixture-time', save=save,
                                     identity=lambda pid: dict(pid=pid, fixture=True))
        self.harness = SimpleNamespace(group_members=lambda pgid: [],
                                       kill_group=lambda pgid: self.cleaned.append(pgid))

    def tearDown(self):
        self.temporary.cleanup()

    def invoke(self, process):
        with patch.object(preparation.subprocess, 'Popen', return_value=process) as popen:
            result = preparation.owned_command(self.guard, self.harness, ['fixture'], {},
                                               self.output, 99, 1, 'owned')
        self.assertEqual(popen.call_args.kwargs['pass_fds'], (99,))
        self.assertTrue(popen.call_args.kwargs['start_new_session'])
        return result

    def test_popen_failure_has_no_foreign_cleanup(self):
        with patch.object(preparation.subprocess, 'Popen', side_effect=OSError('fixture launch failure')):
            with self.assertRaises(OSError):
                preparation.owned_command(self.guard, self.harness, ['fixture'], {}, self.output, 99, 1, 'owned')
        self.assertEqual(self.cleaned, [])
        self.assertEqual(json.loads((self.output / 'owned.json').read_text())['error_type'], 'OSError')

    def test_identity_failure_cleans_known_child(self):
        process = FakeProcess()
        self.guard.identity = lambda pid: None
        with self.assertRaises(RuntimeError):
            self.invoke(process)
        self.assertEqual(self.cleaned, [process.pid])
        self.assertTrue(process.waited)
        self.assertTrue(process.stdout.closed and process.stderr.closed)

    def test_journal_failure_cleans_known_child(self):
        process = FakeProcess()
        self.guard.save = lambda path, value: (_ for _ in ()).throw(OSError('fixture journal failure'))
        with self.assertRaises(OSError):
            self.invoke(process)
        self.assertEqual(self.cleaned, [process.pid])
        self.assertTrue(process.waited)
        self.assertTrue((self.output / 'owned.json').exists())

    def test_timeout_preserves_complete_drained_output(self):
        process = FakeProcess(timeout=True)
        with self.assertRaises(subprocess.TimeoutExpired):
            self.invoke(process)
        self.assertEqual(self.cleaned, [process.pid])
        self.assertEqual((self.output / 'owned.stdout').read_bytes(), b'complete')
        self.assertEqual((self.output / 'owned.stderr').read_bytes(), b'complete-stderr')
        self.assertEqual(json.loads((self.output / 'owned.json').read_text())['error_type'], 'TimeoutExpired')

    def test_orphan_compiler_child_is_fatal(self):
        process = FakeProcess()
        self.harness.group_members = lambda pgid: [dict(pid=123457)]
        with self.assertRaises(AssertionError):
            self.invoke(process)
        self.assertEqual(self.cleaned, [process.pid])

    def test_cleanup_failure_is_fatal_and_waits(self):
        process = FakeProcess()
        self.harness.kill_group = lambda pgid: (_ for _ in ()).throw(RuntimeError('fixture cleanup failure'))
        with self.assertRaisesRegex(RuntimeError, 'cleanup failed'):
            self.invoke(process)
        self.assertTrue(process.waited)
        self.assertEqual(json.loads((self.output / 'owned.json').read_text())['cleanup_error'], 'fixture cleanup failure')

    def test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group(self):
        class LiveProcess(FakeProcess):
            def __init__(self):
                super().__init__()
                self.killed = False
                self.wait_timeouts = []
            def kill(self):
                self.killed = True
                self.returncode = -9
            def wait(self, timeout):
                self.wait_timeouts.append(timeout)
                self.waited = True
                if self.returncode is None:
                    raise subprocess.TimeoutExpired(['fixture-live'], timeout)
                return self.returncode
            def communicate(self, timeout):
                if self.returncode is None:
                    raise subprocess.TimeoutExpired(['fixture-live'], timeout, output=b'actual-before-kill')
                return b'actual-drained-after-kill', b'actual-stderr'
        process = LiveProcess()
        self.guard.identity = lambda pid: None
        self.harness.kill_group = lambda pgid: (_ for _ in ()).throw(RuntimeError('fixture live cleanup failure'))
        with patch.object(preparation.os, 'killpg', side_effect=OSError('fixture group signal failure')) as killpg:
            with self.assertRaisesRegex(RuntimeError, 'cleanup failed'):
                self.invoke(process)
        killpg.assert_called_once_with(process.pid, preparation.signal.SIGKILL)
        self.assertTrue(process.killed and process.waited)
        self.assertEqual(process.wait_timeouts, [5])
        self.assertTrue(process.stdout.closed and process.stderr.closed)
        receipt = json.loads((self.output / 'owned.json').read_text())
        self.assertEqual(receipt['final_exit_status'], -9)
        self.assertEqual([item['stage'] for item in receipt['cleanup_errors']],
                         ['owned-new-PGID-cleanup', 'live-owned-PGID-fallback'])
        self.assertEqual(receipt['error'], 'Owned child identity missing while still live')
        self.assertEqual((self.output / 'owned.stdout').read_bytes(), b'actual-drained-after-kill')


class ProbeRestorationChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.output = Path(self.temporary.name)
        self.old_validate = lambda *args: None
        self.old_input = (Path('original-stdin'), Path('original-reference'))
        self.harness = SimpleNamespace(WORK={'knucleotide': (100, 1000)},
                                       INPUTS={'knucleotide': self.old_input}, validate=self.old_validate)
        self.row = dict(benchmark='knucleotide', id='fixture-row')
        self.report = dict(attempts=[], active=None)
        self.guard = SimpleNamespace(OUT=self.output, REPORT=self.output / 'report.json', save=save)

    def tearDown(self):
        self.temporary.cleanup()

    def invoke(self, case, data=b'>THREE\nACGT', trace=None):
        return preparation.probe(self.guard, self.harness, self.row, 4, data, b'expected', {},
                                 self.output, 99, self.report, case, trace=trace)

    def assert_restored(self):
        self.assertEqual(self.harness.WORK['knucleotide'], (100, 1000))
        self.assertIs(self.harness.validate, self.old_validate)
        self.assertEqual(self.harness.INPUTS['knucleotide'], self.old_input)

    def test_setup_failure_restores_present_state(self):
        with patch.object(preparation, 'sha', side_effect=OSError('fixture expected pin failure')):
            with self.assertRaises(OSError):
                self.invoke('setup')
        self.assert_restored()

    def test_absent_input_key_is_removed_on_failure(self):
        self.harness.INPUTS.pop('knucleotide')

        def fail(*args):
            self.harness.INPUTS['knucleotide'] = ('introduced', 'introduced')
            raise RuntimeError('fixture parent failure')

        self.guard.one_attempt = fail
        with self.assertRaises(RuntimeError):
            self.invoke('absent', data=None)
        self.assertNotIn('knucleotide', self.harness.INPUTS)
        self.assertIs(self.harness.validate, self.old_validate)
        self.assertFalse(self.report['attempts'][0]['qualified'])
        self.assertNotIn('seconds', self.report['attempts'][0])

    def test_parent_failure_archives_actual_sampler_bytes(self):
        raw = self.output / 'attempts/001-fixture-row.json'
        original = b'{"seconds": 0.25, "exit_status": 0, "error": "actual sampler error"}'

        def fail(*args):
            raw.parent.mkdir()
            raw.write_bytes(original)
            raise RuntimeError('fixture parent journal failure')

        self.guard.one_attempt = fail
        with self.assertRaises(RuntimeError):
            self.invoke('parent')
        self.assert_restored()
        self.assertEqual(raw.read_bytes(), original)
        attempt = self.report['attempts'][0]
        self.assertEqual(attempt['seconds'], 0.25)
        self.assertEqual(attempt['execution_failure_before_preparation_error']['error'], 'actual sampler error')
        self.assertFalse(attempt['qualified'])
        self.assertIsNone(self.report['active'])

    def test_trace_failure_archives_and_stops(self):
        self.guard.one_attempt = lambda *args: dict(qualified=True, seconds=0.25)
        with self.assertRaises(AssertionError):
            self.invoke('trace', trace=lambda directory: (_ for _ in ()).throw(AssertionError('fixture trace mismatch')))
        self.assert_restored()
        attempt = self.report['attempts'][0]
        self.assertFalse(attempt['qualified'])
        self.assertTrue(attempt['output_qualified_before_source_work'])


if __name__ == '__main__':
    unittest.main()
