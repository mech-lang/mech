import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "run-gate-b-benchmarks.py"
SPEC = importlib.util.spec_from_file_location("run_gate_b_benchmarks", SCRIPT)
RUNNER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = RUNNER
SPEC.loader.exec_module(RUNNER)


class GateBBenchmarkRunnerTests(unittest.TestCase):
    def parse_args(self, *arguments):
        with patch.object(sys, "argv", [str(SCRIPT), *arguments]):
            return RUNNER.parse_args()

    def test_defaults_use_controlled_sample_protocol(self):
        arguments = self.parse_args()
        self.assertEqual(arguments.sample_size, 10)
        self.assertEqual(arguments.warm_up_time, 1.0)
        self.assertEqual(arguments.measurement_time, 3.0)

    def test_controlled_environment_forces_one_thread(self):
        environment = RUNNER.controlled_environment(
            {"OPENBLAS_NUM_THREADS": "12", "UNCHANGED": "yes"}
        )
        self.assertEqual(environment["UNCHANGED"], "yes")
        for variable in RUNNER.THREAD_VARIABLES:
            self.assertEqual(environment[variable], "1")

    def test_relative_target_directory_is_resolved_from_repo(self):
        target = RUNNER.cargo_target_directory(
            {"CARGO_TARGET_DIR": "build/gate-b-target"}
        )
        self.assertEqual(target, (RUNNER.ROOT / "build/gate-b-target").resolve())

    def test_criterion_samples_are_episode_durations(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory)
            sample = (
                target
                / "criterion/gate_b_rust-kernel/1/new/sample.json"
            )
            sample.parent.mkdir(parents=True)
            sample.write_text(
                json.dumps({"iters": [2, 4, 8], "times": [20, 80, 80]}),
                encoding="utf-8",
            )
            result = RUNNER.criterion_samples(target)["gate_b/rust-kernel/1"]
            self.assertEqual(result["sample_count"], 3)
            self.assertEqual(result["median_episode_ns"], 10.0)
            self.assertEqual(result["p95_episode_ns"], 19.0)

    def test_clear_removes_only_sanitized_gate_b_results(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory)
            gate_b = target / "criterion/gate_b_rust-epoch"
            unrelated = target / "criterion/gate_a"
            gate_b.mkdir(parents=True)
            unrelated.mkdir()
            RUNNER.clear_gate_b_criterion_results(target)
            self.assertFalse(gate_b.exists())
            self.assertTrue(unrelated.exists())

    def test_probe_parser_keys_lane_and_scale(self):
        payload = {
            "lane": "rust-epoch",
            "instances": 8,
            "turns": 4096,
        }
        output = "prefix\nGATE_B_SAMPLE " + json.dumps(payload) + "\n"
        self.assertEqual(
            RUNNER.parse_probe_samples(output)[("rust-epoch", 8)], payload
        )

    def test_untimed_probe_merge_preserves_timed_allocation(self):
        timed = {}
        structural = {}
        for instances in RUNNER.SCALED_INSTANCES:
            key = ("mech-legacy-atomic", instances)
            timed[key] = {"allocation_count": 7}
            structural[key] = {
                "commit_runtime_call_count": 4096,
                "legacy_journal_capture_count": 12,
            }
        full_key = ("mech-legacy-atomic-full-write", 1)
        timed[full_key] = {"allocation_count": 9}
        structural[full_key] = {
            "commit_runtime_call_count": 4096,
            "legacy_journal_capture_count": 13,
        }
        for instances in RUNNER.SCALED_INSTANCES:
            key = ("mech-resident-kernel", instances)
            timed[key] = {"allocation_count": 0}
            structural[key] = {
                field: (1 if field == "publication_store_count" else 0)
                for field in RUNNER.STRUCTURAL_FIELDS
            }
        resident_full = ("mech-resident-kernel-full-write", 1)
        timed[resident_full] = {"allocation_count": 0}
        structural[resident_full] = {
            field: (1 if field == "publication_store_count" else 0)
            for field in RUNNER.STRUCTURAL_FIELDS
        }
        merged = RUNNER.merge_structural_probes(timed, structural)
        self.assertEqual(merged[full_key]["allocation_count"], 9)
        self.assertEqual(merged[full_key]["commit_runtime_call_count"], 4096)
        self.assertEqual(
            merged[("mech-resident-kernel", 64)]["publication_store_count"], 1
        )

    def test_frozen_base_requires_exact_merge_base(self):
        with patch.object(
            RUNNER,
            "command_output",
            return_value=RUNNER.FROZEN_BASE,
        ):
            self.assertIsNone(
                RUNNER.frozen_base_error("a" * 40, RUNNER.FROZEN_B0_BRANCH)
            )
        with patch.object(RUNNER, "command_output", return_value="b" * 40):
            error = RUNNER.frozen_base_error(
                "a" * 40, RUNNER.FROZEN_B0_BRANCH
            )
        self.assertIn("not based on frozen base", error)

        with patch.object(
            RUNNER, "command_output", return_value=RUNNER.FROZEN_B1_BASE
        ):
            self.assertIsNone(
                RUNNER.frozen_base_error("c" * 40, RUNNER.FROZEN_B1_BRANCH)
            )

    def test_lane_record_normalizes_by_host_turn_not_instance(self):
        probe = {
            "turns": 4096,
            "allocation_count": 4096,
            "allocated_bytes": 8192,
            "correctness": True,
            "quantized_state_hash": "a" * 64,
        }
        record = RUNNER.lane_record(
            "fixture", 64, 10, 40960.0, 81920.0, probe, "b" * 64
        )
        self.assertEqual(record["timing"]["median_ns_per_turn"], 10.0)
        self.assertEqual(record["allocation"]["allocations_per_turn"], 1.0)

    def test_legacy_denominator_uses_primary_scale(self):
        lanes = [
            {
                "lane": "rust-epoch",
                "instances": 1,
                "timing": {"median_ns_per_turn": 25.0},
            },
            {
                "lane": "rust-epoch",
                "instances": 8,
                "timing": {"median_ns_per_turn": 500.0},
            },
            {
                "lane": "mech-legacy-atomic",
                "instances": 1,
                "timing": {"median_ns_per_turn": 100.0},
            },
        ]
        derived = RUNNER.legacy_denominator(lanes)
        self.assertEqual(derived["legacy_denominator_ns_per_turn"], 75.0)
        self.assertTrue(derived["positive"])

    def test_b1_progression_uses_same_session_primary_medians(self):
        lanes = [
            {
                "lane": "mech-resident-kernel",
                "instances": 1,
                "timing": {"median_ns_per_turn": 21.0},
            },
            {
                "lane": "rust-kernel",
                "instances": 1,
                "timing": {"median_ns_per_turn": 20.0},
            },
            {
                "lane": "rust-epoch",
                "instances": 1,
                "timing": {"median_ns_per_turn": 22.0},
            },
        ]
        progression = RUNNER.b1_progression(lanes)
        self.assertEqual(progression["limit_ns_per_turn"], 23.1)
        self.assertTrue(progression["passed"])

    def test_explicit_machine_label_is_preserved(self):
        self.assertEqual(
            RUNNER.hardware_description("  controlled-host-model  "),
            "controlled-host-model",
        )

    def test_environment_does_not_change_cli_sample_size(self):
        with patch.dict(os.environ, {"MECH_GATE_B_SAMPLE_SIZE": "1"}):
            self.assertEqual(self.parse_args().sample_size, 10)


if __name__ == "__main__":
    unittest.main()
