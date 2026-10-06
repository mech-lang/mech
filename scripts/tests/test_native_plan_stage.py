#!/usr/bin/env python3
"""Exercise the CI stage boundary without running Cargo or changing the global PATH."""

import importlib.util
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "run-native-plan-stage.py"
SPEC = importlib.util.spec_from_file_location("native_plan_stage", SCRIPT)
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)


class NativePlanStageTests(unittest.TestCase):
    def fixture_stage_command(self, root, source, timeout=5, *, ready=None, stage="fixture"):
        if ready is not None:
            # Freeze the injected deadline during fixture startup. Expire it
            # only after the actual descendant or complete payload is ready.
            # A separate real-time guard bounds a broken fixture; mandatory
            # readiness assertions still fail if that guard is reached.
            launcher = f"""
import importlib.util, sys, time, types
from pathlib import Path
spec = importlib.util.spec_from_file_location('stage', {str(SCRIPT)!r})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
origin = time.monotonic()
expired = None
def clock():
    global expired
    now = time.monotonic()
    if expired is None and (Path({str(ready)!r}).exists() or now - origin >= 10):
        expired = now
    return origin if expired is None else origin + {timeout!r} + 1 + now - expired
module.time = types.SimpleNamespace(monotonic=clock, sleep=time.sleep)
sys.exit(module.run_stage({stage!r}, [sys.executable, '-c', {source!r}], {timeout!r}, Path({str(root)!r})))
"""
            return [sys.executable, "-c", launcher]
        return [sys.executable, str(SCRIPT), "--stage", stage, "--timeout-secs", str(timeout),
                "--log-dir", str(root), "--", sys.executable, "-c", source]

    def run_fixture(self, root, source, timeout=5, *, ready=None):
        return subprocess.run(
            self.fixture_stage_command(root, source, timeout, ready=ready),
            capture_output=True, text=True, timeout=15,
        )

    def assert_dead(self, pid):
        observed = subprocess.run(["ps", "-p", str(pid), "-o", "stat="],
                                  capture_output=True, text=True, timeout=3)
        self.assertIn(observed.returncode, (0, 1), observed.stderr)
        self.assertFalse(observed.stderr.strip(), observed.stderr)
        self.assertTrue(not observed.stdout.strip() or observed.stdout.strip().startswith("Z"),
                        f"fixture process still running: {observed.stdout}")

    def test_nonzero_preserves_output_summary_and_resource_snapshots(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = self.run_fixture(root, "import sys; print('fixture failed', flush=True); sys.exit(7)")
            self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
            self.assertIn("fixture failed", (root / "fixture.log").read_text())
            summary = json.loads((root / "fixture.summary.json").read_text())
            self.assertEqual(summary["reason"], "exit")
            self.assertEqual(summary["command_exit_status"], 7)
            resources = (root / "fixture.resources.log").read_text()
            self.assertIn("event=before", resources)
            self.assertIn("event=after", resources)
            self.assertIn("memory.events", resources)
            self.assertIn("memory.pressure", resources)
            self.assertIn("process_tree_", resources)

    @unittest.skipUnless(os.name == "posix", "CI stage runner is used on Unix")
    def test_stalled_child_has_bounded_retained_diagnostic_and_dead_process_tree(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ready = root / "descendant.ready"
            descendant_source = (
                "import os,time; from pathlib import Path; time.sleep(1); "
                f"Path({str(ready)!r}).write_text(str(os.getpid())); time.sleep(60)"
            )
            source = (
                "import subprocess, sys, time; "
                f"child = subprocess.Popen([sys.executable, '-c', {descendant_source!r}]); "
                "print('fixture-descendant=' + str(child.pid), flush=True); time.sleep(60)"
            )
            result = self.run_fixture(root, source, timeout=0.3, ready=ready)
            self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
            log = (root / "fixture.log").read_text()
            self.assertIn("diagnostic=timeout", log)
            summary = json.loads((root / "fixture.summary.json").read_text())
            self.assertEqual(summary["reason"], "timeout")
            self.assertLess(summary["elapsed_seconds"], 10)
            self.assertIn("event=before-timeout-cleanup", (root / "fixture.resources.log").read_text())
            descendant = int(re.search(r"fixture-descendant=(\d+)", log).group(1))
            self.assertEqual(int(ready.read_text()), descendant)
            self.assert_dead(descendant)

    @unittest.skipUnless(os.name == "posix", "CI stage runner is used on Unix")
    def test_undrained_console_cannot_prevent_timeout_or_complete_disk_retention(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pid_file = root / "child.pid"
            ready = root / "payload.ready"
            payload = b"BEGIN-BLOCKED-CONSOLE\n" + b"x" * 1048576 + b"\nEND-BLOCKED-CONSOLE\n"
            source = (
                "import os,sys,time; from pathlib import Path; "
                "time.sleep(1); "
                f"Path({str(pid_file)!r}).write_text(str(os.getpid())); "
                "sys.stdout.buffer.write(b'BEGIN-BLOCKED-CONSOLE\\n' + b'x'*1048576 + "
                "b'\\nEND-BLOCKED-CONSOLE\\n'); sys.stdout.buffer.flush(); "
                f"Path({str(ready)!r}).write_text('complete'); time.sleep(60)"
            )
            process = subprocess.Popen(
                self.fixture_stage_command(root, source, timeout=0.5, ready=ready,
                                           stage="blocked-console"),
                # Neither console pipe is drained until supervision completes.
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
            )
            completed = False
            try:
                status = process.wait(timeout=10)
                completed = True
                self.assertEqual(status, 124)
                log = (root / "blocked-console.log").read_bytes()
                self.assertIn(payload, log)
                self.assertEqual(ready.read_text(), "complete")
                self.assertIn(b"diagnostic=timeout", log)
                summary = json.loads((root / "blocked-console.summary.json").read_text())
                self.assertEqual(summary["reason"], "timeout")
                self.assertLess(summary["elapsed_seconds"], 10)
                child = int(pid_file.read_text())
                self.assert_dead(child)
            finally:
                if not completed:
                    # The regression's own safety coordinator cleans both
                    # sessions even when the stage deadline is defeated.
                    for pid in ([int(pid_file.read_text())] if pid_file.exists() else []) + [process.pid]:
                        try:
                            os.killpg(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                    process.wait(timeout=5)
                process.stdout.close()
                process.stderr.close()

    @unittest.skipUnless(os.name == "posix", "CI stage runner is used on Unix")
    def test_ready_descendants_are_cleaned_after_success_and_nonzero_leader_exit(self):
        for exit_code in (0, 7):
            with self.subTest(exit_code=exit_code), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                ready = root / "descendant.ready"
                release = root / "perform-work"
                worked = root / "descendant-worked"
                child_source = f"""
import json, os, time
from pathlib import Path
Path({str(ready)!r}).write_text(json.dumps([os.getpid(), os.getpgrp()]))
while not Path({str(release)!r}).exists():
    time.sleep(0.01)
Path({str(worked)!r}).write_text('escaped cleanup')
"""
                source = f"""
import subprocess, sys, time
from pathlib import Path
subprocess.Popen([sys.executable, '-c', {child_source!r}])
while not Path({str(ready)!r}).exists():
    time.sleep(0.01)
sys.exit({exit_code})
"""
                try:
                    result = self.run_fixture(root, source, timeout=10)
                    self.assertEqual(result.returncode, exit_code, result.stdout + result.stderr)
                    descendant, group = json.loads(ready.read_text())
                    self.assert_dead(descendant)
                    release.touch()
                    self.assertFalse(worked.exists())
                    summary = json.loads((root / "fixture.summary.json").read_text())
                    self.assertEqual(summary["command_exit_status"], exit_code)
                    self.assertEqual(summary["owned_group_cleanup"], "complete")
                finally:
                    if ready.exists():
                        _, group = json.loads(ready.read_text())
                        try:
                            os.killpg(group, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    @unittest.skipUnless(os.name == "posix", "CI stage runner is used on Unix")
    def test_startup_deadline_can_expire_before_fixture_readiness(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ready = root / "never-ready"
            source = f"import time; from pathlib import Path; time.sleep(60); Path({str(ready)!r}).touch()"
            result = self.run_fixture(root, source, timeout=0.3)
            self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
            self.assertFalse(ready.exists())
            summary = json.loads((root / "fixture.summary.json").read_text())
            self.assertEqual(summary["reason"], "timeout")
            self.assertEqual(summary["owned_group_cleanup"], "complete")

    def test_compiled_test_executable_requires_the_selected_target_and_one_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "planning-test"
            executable.touch()
            log = root / "compile.log"
            log.write_text(json.dumps({
                "reason": "compiler-artifact", "target": {"name": "planning"},
                "profile": {"test": True}, "executable": str(executable),
            }) + "\n")
            selected = root / "planning-executable"
            STAGE.record_test_executable(log, "planning", selected)
            self.assertEqual(selected.read_text().strip(), str(executable.resolve()))
            with self.assertRaisesRegex(ValueError, "expected one compiled test executable"):
                STAGE.record_test_executable(log, "isolated_process", selected)

    def test_execution_invokes_recorded_binary_without_cargo(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "prepared-test"
            executable.write_text(f"#!{sys.executable}\nimport sys\nprint('prepared fixture', sys.argv[1:])\n")
            executable.chmod(0o755)
            selected = root / "planning-executable"
            selected.write_text(str(executable) + "\n")
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--stage", "execute", "--timeout-secs", "5",
                 "--log-dir", str(root), "--test-executable-file", str(selected),
                 "--", "--test-threads=1", "--nocapture"],
                capture_output=True, text=True, timeout=15,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            log = (root / "execute.log").read_text()
            self.assertIn("prepared fixture ['--test-threads=1', '--nocapture']", log)
            self.assertNotIn("cargo", log)

    def test_launch_failure_has_retained_diagnostic_and_nonzero_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--stage", "launch", "--timeout-secs", "5",
                 "--log-dir", str(root), "--", str(root / "missing-command")],
                capture_output=True, text=True, timeout=15,
            )
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("diagnostic=stage-error", (root / "launch.log").read_text())
            summary = json.loads((root / "launch.summary.json").read_text())
            self.assertEqual(summary["reason"], "stage-error")
            self.assertEqual(summary["command_exit_status"], 1)


if __name__ == "__main__":
    unittest.main()
