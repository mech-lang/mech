"""Mocked failure-path fixtures; sends no signal and starts no native process."""
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
import owned_service_v7 as service


def write_json(path, payload):
    with path.open('x') as stream:
        json.dump(payload, stream)


class OwnedService(unittest.TestCase):
    def invoke(self, process, identity, kill_error=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        output = Path(temporary.name)
        preparation = SimpleNamespace(HERE=output, new_json=write_json)
        guard = SimpleNamespace(now=lambda: 'fixture-time', save=write_json)
        contexts = (
            patch.object(service.subprocess, 'Popen', return_value=process),
            patch.object(service, 'identity', return_value=identity),
            patch.object(service.os, 'getpgid', return_value=process.pid),
            patch.object(service, 'group_members', return_value=[]),
            patch.object(service, 'kill_owned_group', side_effect=kill_error),
        )
        for context in contexts:
            context.start(); self.addCleanup(context.stop)
        return output, lambda: service.owned_command(preparation, guard, None,
            ['fixture-only'], {}, output, 123, 30, 'mocked-attempt')

    def process(self):
        process = Mock(pid=424242, returncode=None)
        process.poll.return_value = None
        def finish(*, timeout):
            process.returncode = 0
            return 0
        process.wait.side_effect = finish
        return process

    def test_success_requires_bounded_reap_and_empty_group(self):
        process = self.process()
        output, invoke = self.invoke(process, {'pid': process.pid, 'process_listing': 'fixture'})
        result = invoke()
        self.assertEqual([call.kwargs['timeout'] for call in process.wait.call_args_list], [30, 5])
        self.assertIs(result['detail']['cleanup_complete'], True)
        self.assertEqual(result['detail']['remaining_group_members'], [])
        self.assertEqual(json.loads((output / 'mocked-attempt.json').read_text())['final_exit_status'], 0)

    def test_failed_group_kill_and_wait_preserve_bounded_receipt(self):
        process = self.process()
        def bounded_wait(*, timeout):
            if process.wait.call_count < 3:
                raise subprocess.TimeoutExpired('fixture-only', timeout)
            process.returncode = -9
            return -9
        process.wait.side_effect = bounded_wait
        output, invoke = self.invoke(process, {'pid': process.pid}, RuntimeError('fixture cleanup refusal'))
        with self.assertRaisesRegex(RuntimeError, 'bounded attempt journal retained'):
            invoke()
        self.assertEqual([call.kwargs['timeout'] for call in process.wait.call_args_list], [30, 5, 5])
        process.kill.assert_called_once_with()
        receipt = json.loads((output / 'mocked-attempt.json').read_text())
        self.assertIs(receipt['cleanup_complete'], False)
        self.assertEqual(receipt['error_type'], 'TimeoutExpired')
        self.assertEqual([entry['stage'] for entry in receipt['cleanup_errors']], ['group-kill', 'leader-wait'])
        self.assertIs(receipt['known_leader_fallback_reaped'], True)
        self.assertIs(receipt['leader_reaped'], True)
        self.assertEqual(receipt['final_exit_status'], -9)

    def test_missing_live_identity_still_cleans_and_journals(self):
        process = self.process()
        output, invoke = self.invoke(process, None)
        with self.assertRaisesRegex(AssertionError, 'live child identity missing'):
            invoke()
        self.assertEqual([call.kwargs['timeout'] for call in process.wait.call_args_list], [5])
        receipt = json.loads((output / 'mocked-attempt.json').read_text())
        self.assertEqual(receipt['error_type'], 'AssertionError')
        self.assertIs(receipt['cleanup_complete'], True)


if __name__ == '__main__': unittest.main()
