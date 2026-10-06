"""Cargo metadata has its own deadline after native project generation."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "native_application_graphs", ROOT / "scripts/check-native-application-graphs.py"
)
GRAPHS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GRAPHS)


class NativeApplicationMetadataTests(unittest.TestCase):
    def test_timeout_terminates_process_tree_and_identifies_stage_and_project(self):
        process = Mock()
        command = ["cargo", "metadata"]
        process.communicate.side_effect = subprocess.TimeoutExpired(command, 120)
        label = "stage=cargo metadata project=generated_native_literal path=/fixture"
        with patch.object(GRAPHS, "METADATA_TIMEOUT_SECONDS", 120):
            with patch.object(GRAPHS.subprocess, "Popen", return_value=process) as spawn:
                with patch.object(GRAPHS, "terminate_process_tree") as terminate:
                    with self.assertRaisesRegex(RuntimeError, label + " exceeded 120s"):
                        GRAPHS.execute(command, label=label)
        process.communicate.assert_called_once_with(timeout=120)
        terminate.assert_called_once_with(process)
        self.assertEqual(spawn.call_args.kwargs["start_new_session"], sys.platform != "win32")

    def test_real_stalled_metadata_process_is_terminated_and_reaped(self):
        label = "stage=cargo metadata project=stalled_fixture"
        with patch.object(
            GRAPHS, "terminate_process_tree", wraps=GRAPHS.terminate_process_tree
        ) as terminate:
            try:
                with patch.object(GRAPHS, "METADATA_TIMEOUT_SECONDS", 0.05):
                    with self.assertRaisesRegex(RuntimeError, label + " exceeded 0.05s"):
                        GRAPHS.execute(
                            [sys.executable, "-c", "import time; time.sleep(60)"],
                            label=label,
                        )
            finally:
                # Keep a failing cleanup test from leaking its owned fixture.
                if terminate.call_args is not None:
                    process = terminate.call_args.args[0]
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
        terminate.assert_called_once()
        self.assertIsNotNone(terminate.call_args.args[0].poll())
        self.assertTrue(terminate.call_args.args[0].stdout.closed)
        self.assertTrue(terminate.call_args.args[0].stderr.closed)

    def test_nonpositive_metadata_deadline_is_rejected_before_spawning(self):
        with patch.object(GRAPHS, "METADATA_TIMEOUT_SECONDS", 0):
            with patch.object(GRAPHS.subprocess, "Popen") as spawn:
                with self.assertRaisesRegex(RuntimeError, "METADATA_TIMEOUT_SECS must be positive"):
                    GRAPHS.execute(["cargo", "metadata"], label="stage=cargo metadata")
        spawn.assert_not_called()

    def test_metadata_stdout_is_separate_from_diagnostic_stderr(self):
        output = GRAPHS.execute(
            [sys.executable, "-c", "import sys; print('{}'); print('diagnostic', file=sys.stderr)"],
            label="stage=cargo metadata project=fixture",
        )
        self.assertEqual(json.loads(output), {})

    def test_validate_project_labels_metadata_with_binary_and_path(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Path(directory)
            (project / "build-plan.json").write_text(
                json.dumps({"binary_name": "generated_native_literal", "runtime_functions": []}),
                encoding="utf-8",
            )
            with patch.object(GRAPHS, "execute", side_effect=RuntimeError("stop at metadata")) as execute:
                with self.assertRaisesRegex(RuntimeError, "stop at metadata"):
                    GRAPHS.validate_project(project)
            execute.assert_called_once()
            self.assertEqual(
                execute.call_args.kwargs["label"],
                f"stage=cargo metadata project=generated_native_literal path={project}",
            )


if __name__ == "__main__":
    unittest.main()
