import importlib.util
import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "run-gate-a-benchmarks.py"
SPEC = importlib.util.spec_from_file_location("run_gate_a_benchmarks", SCRIPT)
RUNNER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = RUNNER
SPEC.loader.exec_module(RUNNER)


class GateABenchmarkRunnerTests(unittest.TestCase):
    def parse_args(self, *arguments):
        with patch.object(sys, "argv", [str(SCRIPT), *arguments]):
            return RUNNER.parse_args()

    def test_extended_sweep_is_opt_in(self):
        self.assertFalse(self.parse_args().extended)
        self.assertTrue(self.parse_args("--extended").extended)

    def test_environment_does_not_implicitly_enable_extended_sweep(self):
        with patch.dict(os.environ, {"MECH_GATE_A_EXTENDED": "1"}):
            self.assertFalse(self.parse_args().extended)

    def test_relative_target_directory_is_resolved_from_cargo_cwd(self):
        target = RUNNER.cargo_target_directory(
            {"CARGO_TARGET_DIR": "build/gate-a-target"}
        )
        self.assertEqual(target, (RUNNER.ROOT / "build/gate-a-target").resolve())

    def test_absolute_target_directory_is_preserved(self):
        target = RUNNER.cargo_target_directory(
            {"CARGO_TARGET_DIR": "/tmp/gate-a-target"}
        )
        self.assertEqual(target, Path("/tmp/gate-a-target").resolve())

    def test_explicit_machine_label_is_preserved(self):
        self.assertEqual(
            RUNNER.hardware_description("  controlled-host-model  "),
            "controlled-host-model",
        )

    def test_generic_processor_requires_machine_label(self):
        with patch.object(RUNNER.sys, "platform", "linux"), patch.object(
            RUNNER.platform, "processor", return_value="arm64"
        ), patch.object(RUNNER.platform, "machine", return_value="arm64"):
            with self.assertRaisesRegex(ValueError, "--machine-label"):
                RUNNER.hardware_description()

    def test_darwin_hardware_overview_supplies_specific_identity(self):
        overview = (
            "Model Name: MacBook Air\n"
            "Model Identifier: Mac15,13\n"
            "Chip: Apple M3\n"
        )

        def fake_output(command):
            if command[0] == "sysctl":
                raise RUNNER.subprocess.CalledProcessError(1, command)
            return overview

        with patch.object(RUNNER.sys, "platform", "darwin"), patch.object(
            RUNNER, "command_output", side_effect=fake_output
        ):
            self.assertEqual(
                RUNNER.hardware_description(),
                "MacBook Air, Mac15,13, Apple M3",
            )


if __name__ == "__main__":
    unittest.main()
