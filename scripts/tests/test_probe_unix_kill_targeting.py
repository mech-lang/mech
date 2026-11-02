#!/usr/bin/env python3
"""Require syscall blocking and exact safe-form targets without delivering signals."""

import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("probe", Path(__file__).parents[1] / "probe-unix-kill-targeting.py")
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class KillTargetingProbeTests(unittest.TestCase):
    def test_every_probe_blocks_syscalls_and_uses_only_signal_zero(self):
        for target in (1_443_247, 2_345_678, 91_234):
            for separated in (False, True):
                command = PROBE.probe_command("/usr/bin/strace", "/usr/bin/kill", target, separated)
                self.assertEqual(command[:6], ["/usr/bin/strace", "-e", "trace=kill", "-e", "inject=kill:error=EPERM", "/usr/bin/kill"])
                self.assertEqual(command[6:], ["-0", *(["--"] if separated else []), str(-target)])

    def test_trace_requires_injected_eperm_for_signal_zero(self):
        self.assertEqual(PROBE.blocked_targets("kill(-1443247, 0) = -1 EPERM (Operation not permitted) (INJECTED)\n"), [-1443247])
        self.assertEqual(PROBE.blocked_targets("kill(-1, 0) = -1 EPERM (Operation not permitted) (INJECTED)\n"), [-1])
        for trace in ("", "kill(-1443247, 0) = -1 EPERM (Operation not permitted)",
                      "kill(-1443247, SIGTERM) = -1 EPERM (INJECTED)"):
            with self.assertRaisesRegex(ValueError, "syscall-blocked signal-zero"):
                PROBE.blocked_targets(trace)


if __name__ == "__main__":
    unittest.main()
