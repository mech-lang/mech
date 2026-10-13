"""Unexecuted mocked descriptor ownership/cleanup; no child or signal is real."""
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import native_fd_observer as fd


class ImmediateThread:
    def __init__(self, target, args, **kwargs):
        self.target, self.args = target, args
    def start(self):
        self.target(*self.args)
    def join(self, timeout=None):
        pass
    def is_alive(self):
        return False


class StopAfterObservation:
    def is_set(self):
        return False
    def set(self):
        pass
    def wait(self, timeout):
        return True


class ObserverChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='clojure-fd-owned-mock-')
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.stream = (self.base / 'synthetic-lock').open('a+')
        self.addCleanup(self.stream.close)
        self.descriptor = self.stream.fileno()
        self.command = ['saved-java', '-cp', 'isolated-classes:vendor-jars', 'subject', '9']
        self.killed, self.processes = [], {}
        self.timeout = False
        self.group_error = False
        identity = os.fstat(self.descriptor)
        self.stdout = (f'p123\ncjava\nf{self.descriptor}u\nD{hex(identity.st_dev)}\ni{identity.st_ino}\nn/tmp/mech-clbg-timing.lock\n').encode()
        owner = self
        class Process:
            def __init__(self, pid, argv):
                self.pid, self.args, self.returncode = pid, argv, None
                self.stdout, self.stderr = SimpleNamespace(close=lambda: None), SimpleNamespace(close=lambda: None)
            def poll(self):
                return self.returncode
            def communicate(self, timeout):
                if self.pid == 101 and owner.group_error:
                    raise RuntimeError('fixture owned group reconciliation failed')
                if self.pid == 100 and owner.timeout and self.returncode is None:
                    raise subprocess.TimeoutExpired(self.args, timeout, output=b'actual-partial-stdout', stderr=b'actual-partial-stderr')
                self.returncode = 0 if self.returncode is None else self.returncode
                return (owner.stdout, b'actual-stderr') if self.pid == 100 else (
                    (b' PID PGID STAT COMM\n', b'') if self.pid == 101 else (b'actual-native-output', b''))
            def wait(self, timeout=None):
                self.returncode = 0 if self.returncode is None else self.returncode
                return self.returncode
            def kill(self):
                owner.killed.append(self.pid)
                self.returncode = -9
        def popen(argv, **kwargs):
            self.assertTrue(kwargs['start_new_session'])
            pid = 123 if argv == self.command else 101 if argv[0] == '/bin/ps' else 100
            value = Process(pid, argv)
            self.processes[pid] = value
            return value
        def execute(*args, **kwargs):
            native = self.harness.subprocess.Popen(self.command, start_new_session=True, pass_fds=(self.descriptor,))
            return dict(actual_stdout='native bytes retained', exit_status=native.wait())
        def kill_group(pid):
            self.assertIn(pid, (100, 101, 123), 'Only captured known mocked child groups may be cleaned')
            self.killed.append(pid)
            self.processes[pid].returncode = -9
        def group_members(pid):
            if self.group_error:
                raise RuntimeError('fixture owned group reconciliation failed')
            return []
        self.harness = SimpleNamespace(execute=execute, subprocess=SimpleNamespace(Popen=popen),
            kill_group=kill_group, group_members=group_members)
        self.original_execute, self.original_popen = execute, popen

    def invoke(self):
        # Fake thread/events/Popen ensure this fixture cannot launch or signal.
        def mocked_killpg(pid, signum):
            self.assertIn(pid, (100, 101), 'Descriptor observer may signal only its actual mocked observer groups')
            self.killed.append(pid)
            self.processes[pid].returncode = -9
        with patch.object(fd.threading, 'Thread', ImmediateThread), patch.object(fd.threading, 'Event', StopAfterObservation), patch.object(fd.os, 'killpg', mocked_killpg):
            proof, restore = fd.install(self.harness, self.command, self.descriptor, self.base / 'evidence')
            try:
                result = self.harness.execute()
            finally:
                restore()
        self.assertIs(self.harness.execute, self.original_execute)
        self.assertIs(self.harness.subprocess.Popen, self.original_popen)
        return proof, result

    def test_restores_exact_original_execute_and_popen_after_observed_native(self):
        proof, result = self.invoke()
        self.assertTrue(proof['native_observed'])
        self.assertEqual(proof['errors'], [])
        self.assertEqual(result['actual_stdout'], 'native bytes retained')
        self.assertNotIn('supervisor_error', result)
        self.assertEqual(self.killed, [])

    def test_timeout_retains_actual_raw_bytes_and_cleans_only_known_owned_groups(self):
        self.timeout = True
        proof, result = self.invoke()
        self.assertFalse(proof['native_observed'])
        self.assertTrue(proof['errors'])
        self.assertIn('supervisor_error', result)
        self.assertEqual(result['actual_stdout'], 'native bytes retained')
        self.assertEqual(set(self.killed), {100, 123})
        raw = self.base / 'evidence/0001-lsof.stdout'
        self.assertTrue(raw.is_file())
        # Final bounded drain and initial TimeoutExpired bytes are both actual;
        # no invented success or replacement native stdout is allowed.
        self.assertEqual(raw.read_bytes(), self.stdout)
        self.assertEqual((self.base / 'evidence/0001-lsof.timeout.stdout').read_bytes(), b'actual-partial-stdout')
        self.assertEqual((self.base / 'evidence/0001-lsof.timeout.stderr').read_bytes(), b'actual-partial-stderr')
        self.assertTrue(raw.with_suffix('.json').is_file())

    def test_group_reconciliation_error_is_unqualified_and_preserves_cleanup_receipt(self):
        self.group_error = True
        proof, result = self.invoke()
        self.assertTrue(proof['errors'])
        self.assertIn('supervisor_error', result)
        import json
        receipt = json.loads((self.base / 'evidence/0001-lsof.json').read_text())
        self.assertTrue(receipt['cleanup_errors'])
        self.assertEqual(receipt['remaining_group'], ['unverified-owned-group'])
        self.assertLessEqual(set(self.killed), {100, 101, 123})
