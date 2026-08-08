import copy
import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-gate-b-contract.py"
SPEC = importlib.util.spec_from_file_location("check_gate_b_contract", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


def lane(name, instances):
    is_numpy = name == "numpy-persistent"
    is_epoch = name == "rust-epoch"
    is_full_epoch = name == "rust-epoch-full-write"
    is_legacy = name.startswith("mech-legacy-atomic")
    allocation_count = None if is_numpy else (1 if is_legacy else 0)
    allocated_bytes = None if is_numpy else (8 if is_legacy else 0)
    structural = {
        "candidate_seed_bytes": None if is_numpy else 0,
        "candidate_written_bytes": None,
        "published_buffer_copy_bytes": None if is_numpy else 0,
        "publication_store_count": None if is_numpy else 0,
        "receipt_bytes": None if is_numpy else 0,
        "commit_runtime_call_count": None if is_numpy else 0,
        "legacy_journal_capture_count": None if is_numpy else 0,
        "abort_output_hash": None,
    }
    if is_epoch:
        structural.update(
            {
                "candidate_written_bytes": instances * 96,
                "publication_store_count": 1,
                "receipt_bytes": 64,
            }
        )
    if is_full_epoch:
        structural.update(
            {
                "candidate_written_bytes": 64 * 64 * 8,
                "publication_store_count": 1,
                "receipt_bytes": 64,
                "abort_output_hash": "c" * 64,
            }
        )
    if is_legacy:
        structural.update(
            {
                "candidate_written_bytes": 64 * 64 * 8
                if name.endswith("full-write")
                else 0,
                "commit_runtime_call_count": 4096,
                "legacy_journal_capture_count": 1,
            }
        )
    return {
        "lane": name,
        "instances": instances,
        "sample_count": 10,
        "turns_per_sample": 4096,
        "timing": {
            "median_ns_per_turn": 100.0
            if name == "mech-legacy-atomic"
            else 25.0,
            "p95_ns_per_turn": 125.0,
        },
        "allocation": {
            "allocations_per_turn": None
            if allocation_count is None
            else allocation_count / 4096,
            "allocated_bytes_per_turn": None
            if allocated_bytes is None
            else allocated_bytes / 4096,
            "episode_allocation_count": allocation_count,
            "episode_allocated_bytes": allocated_bytes,
        },
        "correctness": True,
        "quantized_state_hash": (
            "d" * 64
            if is_numpy or name.endswith("full-write")
            else CHECKER.REFERENCE_HASH
        ),
        "reference_quantized_state_hash": CHECKER.REFERENCE_HASH,
        "structural": structural,
    }


def valid_report():
    lanes = []
    for name in (
        "rust-kernel",
        "rust-epoch",
        "mech-legacy-atomic",
        "numpy-persistent",
    ):
        lanes.extend(lane(name, instances) for instances in (1, 8, 64))
    lanes.extend(
        [
            lane("rust-epoch-full-write", 1),
            lane("mech-legacy-atomic-full-write", 1),
        ]
    )
    return {
        "schema_version": 1,
        "gate": "B",
        "phase": "B0-controls",
        "git_commit": "a" * 40,
        "git_branch": "test/resident-ekf-efficacy-contract",
        "machine": {
            "identity": "test machine",
            "os": "test os",
            "architecture": "test architecture",
        },
        "toolchain": {
            "rustc": "rustc test",
            "RUSTFLAGS": "",
            "CARGO_ENCODED_RUSTFLAGS": "",
            "python": "3.test",
            "python_executable": "/test/python",
            "numpy": "test",
            "numpy_config": "test config",
            "blas_lapack_provider": "test blas",
        },
        "thread_environment": {
            variable: "1" for variable in CHECKER.THREAD_VARIABLES
        },
        "trace": {
            "sha256": CHECKER.TRACE_SHA256,
            "file": "ekf-input-v1.bin",
        },
        "workload": {
            "version": "resident-ekf-v1",
            "episode_length": 4096,
            "scaled_instances": [1, 8, 64],
            "scalar": "f64",
            "matrix_storage": "column-major",
        },
        "sample_protocol": {
            "criterion_sample_size": 10,
            "numpy_sample_size": 10,
            "warm_up_seconds": 1.0,
            "measurement_seconds": 3.0,
            "turns_per_sample": 4096,
            "fixture_setup_included_in_timing": False,
            "correctness_included_in_timing": False,
            "profile": "release",
        },
        "benchmark_arguments": ["cargo", "bench"],
        "structural_probe_arguments": ["cargo", "bench", "structural"],
        "raw_criterion_directory": "/test/criterion",
        "raw_output": "/test/timed.log",
        "raw_structural_probe_output": "/test/structural.log",
        "lanes": lanes,
        "derived": {
            "mech_legacy_atomic_ns_per_turn": 100.0,
            "rust_epoch_ns_per_turn": 25.0,
            "legacy_denominator_ns_per_turn": 75.0,
            "positive": True,
        },
        "stop_condition": {
            "name": "positive-legacy-denominator",
            "passed": True,
        },
    }


class GateBContractCheckerTests(unittest.TestCase):
    def test_committed_static_contract_passes(self):
        self.assertEqual(CHECKER.static_contract_errors(), [])

    def test_valid_b0_report_passes(self):
        self.assertEqual(CHECKER.report_contract_errors(valid_report()), [])

    def test_nonpositive_denominator_is_a_hard_error(self):
        report = valid_report()
        for result in report["lanes"]:
            if result["lane"] == "mech-legacy-atomic" and result["instances"] == 1:
                result["timing"]["median_ns_per_turn"] = 20.0
        report["derived"] = {
            "legacy_denominator_ns_per_turn": -5.0,
            "positive": False,
        }
        report["stop_condition"]["passed"] = False
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("non-positive" in error for error in errors))

    def test_missing_lane_is_rejected(self):
        report = valid_report()
        report["lanes"] = report["lanes"][:-1]
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("missing lanes" in error for error in errors))

    def test_raw_epoch_allocation_is_rejected(self):
        report = valid_report()
        result = next(
            lane
            for lane in report["lanes"]
            if lane["lane"] == "rust-epoch" and lane["instances"] == 8
        )
        result["allocation"]["episode_allocation_count"] = 1
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("raw epoch 8 allocates" in error for error in errors))

    def test_wrong_exact_report_commit_is_rejected(self):
        errors = CHECKER.report_contract_errors(valid_report(), "b" * 40)
        self.assertTrue(any("!= expected" in error for error in errors))

    def test_committed_schema_rejects_missing_machine_identity(self):
        report = valid_report()
        del report["machine"]["identity"]
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("schema-required property identity" in error for error in errors))

    def test_admission_check_ignores_import_and_requires_call_before_timing(self):
        source = "use crate::reserve_retained;\npub fn run_episode() {}\nreserve_retained(&ledger);\n"
        self.assertFalse(
            CHECKER.call_occurs_before(
                source, "reserve_retained", "pub fn run_episode"
            )
        )

    def test_prohibited_accelerator_imports_are_visible(self):
        self.assertEqual(
            CHECKER.import_roots("import numpy as np\nfrom numba import njit\n"),
            {"numpy", "numba"},
        )


if __name__ == "__main__":
    unittest.main()
