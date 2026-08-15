import copy
import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "check-gate-b-contract.py"
SPEC = importlib.util.spec_from_file_location("check_gate_b_contract", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


def lane(name, instances, retained_history=0, next_epoch=1):
    is_numpy = name == "numpy-persistent"
    is_epoch = name == "rust-epoch"
    is_full_epoch = name == "rust-epoch-full-write"
    is_resident = name == "mech-resident-kernel"
    is_resident_full = name == "mech-resident-kernel-full-write"
    is_scheduled = name == "mech-resident-scheduled"
    is_turn = name == "mech-resident-turn"
    is_turn_full = name == "mech-resident-turn-full-write"
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
        "dirty_node_count": None,
        "record_preparation_count": None,
        "record_append_count": None,
        "records_retained_before_timing": None,
        "records_appended": None,
        "ledger_records_inspected": None,
        "post_publication_append_infallible": None,
    }
    if is_epoch:
        structural.update(
            {
                "candidate_written_bytes": instances * 96,
                "publication_store_count": 1,
                "receipt_bytes": 64,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_appended": 4096,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }
        )
    if is_full_epoch:
        structural.update(
            {
                "candidate_written_bytes": 64 * 64 * 8,
                "publication_store_count": 1,
                "receipt_bytes": 64,
                "abort_output_hash": "c" * 64,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_appended": 4096,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }
        )
    if is_resident:
        structural.update(
            {
                "candidate_written_bytes": instances * 96,
                "publication_store_count": 1,
            }
        )
    if is_scheduled:
        structural.update(
            {
                "candidate_written_bytes": 96,
                "publication_store_count": 1,
            }
        )
    if is_turn:
        structural.update(
            {
                "candidate_written_bytes": 96,
                "publication_store_count": 1,
                "receipt_bytes": 64,
                "dirty_node_count": 15,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_retained_before_timing": retained_history,
                "records_appended": 4096,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }
        )
    if is_turn_full:
        structural.update(
            {
                "candidate_written_bytes": 64 * 64 * 8,
                "publication_store_count": 1,
                "receipt_bytes": 64,
                "dirty_node_count": 1,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_retained_before_timing": 0,
                "records_appended": 4096,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
                "abort_output_hash": "c" * 64,
            }
        )
    if is_resident_full:
        structural.update(
            {
                "candidate_written_bytes": 64 * 64 * 8,
                "publication_store_count": 1,
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
        "retained_history": retained_history,
        "next_epoch": next_epoch,
        "sample_count": 10,
        "turns_per_sample": 4096,
        "timing": {
            "median_ns_per_turn": 100.0
            if name == "mech-legacy-atomic"
            else 25.0,
            "p95_ns_per_turn": 30.0 if is_turn else 125.0,
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
        "reference_quantized_state_hash": (
            "d" * 64 if name.endswith("full-write") else CHECKER.REFERENCE_HASH
        ),
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


def valid_b1_report():
    report = valid_report()
    report["phase"] = "B1-resident-kernel"
    report["git_branch"] = "feat/engine-resident-ekf-substrate"
    for instances in (1, 8, 64):
        resident = lane("mech-resident-kernel", instances)
        report["lanes"].append(resident)
    report["lanes"].append(lane("mech-resident-kernel-full-write", 1))
    report["b1_progression"] = {
        "resident_kernel_ns_per_turn": 25.0,
        "rust_kernel_ns_per_turn": 25.0,
        "rust_epoch_ns_per_turn": 25.0,
        "resident_kernel_ratio": 1.0,
        "resident_kernel_vs_raw_epoch": 1.0,
        "limit_multiplier": 1.05,
        "limit_ns_per_turn": 26.25,
        "passed": True,
    }
    return report


def valid_b2_report():
    report = valid_b1_report()
    report["phase"] = "B2-resident-turn"
    report["git_branch"] = "perf/runtime-resident-ekf-efficacy"
    report["lanes"].extend(
        [
            lane("mech-resident-scheduled", 1),
            lane("mech-resident-turn", 1),
            lane("mech-resident-turn", 1, retained_history=1_000),
            lane("mech-resident-turn", 1, retained_history=100_000),
            lane("mech-resident-turn", 1, next_epoch=1_000_000_001),
            lane("mech-resident-turn-full-write", 1),
        ]
    )
    report["b2_decision"] = {
        "legacy_gap_closure": 1.0,
        "raw_epoch_ratio": 1.0,
        "executor_tax_ns": 0.0,
        "scheduler_tax_ns": 0.0,
        "recording_tax_ns": 0.0,
        "numpy_ratio": 1.0,
        "tail_ratio": 1.2,
        "history_1k_over_history_0_median_ratio": 1.0,
        "history_100k_over_history_0_median_ratio": 1.0,
        "high_epoch_over_low_epoch_median_ratio": 1.0,
        "hard_gates": {
            "correctness": True,
            "zero_allocation": True,
            "constant_publication": True,
            "no_full_clone": True,
            "history_independent": True,
            "legacy_gap_closure": True,
            "raw_epoch_ratio": True,
            "executor_tax": True,
            "tail_stability": True,
            "post_publication_append_infallible": True,
        },
        "numpy_target": True,
        "decision": "Pass",
        "conditional_attribution": None,
    }
    return report


class GateBContractCheckerTests(unittest.TestCase):
    def test_committed_static_contract_passes(self):
        self.assertEqual(CHECKER.static_contract_errors(), [])

    def test_static_contract_rejects_restored_legacy_atomic_fixture(self):
        retired = (
            CHECKER.ROOT
            / "src/runtime/benches/support/gate_b/legacy_atomic.rs"
        )
        original_exists = Path.exists

        def exists(path):
            return path == retired or original_exists(path)

        with mock.patch.object(Path, "exists", autospec=True, side_effect=exists):
            errors = CHECKER.static_contract_errors()
        self.assertTrue(
            any("legacy-atomic fixture was restored" in error for error in errors)
        )

    def test_static_contract_rejects_restored_legacy_atomic_benchmark_lane(self):
        benchmark = CHECKER.ROOT / "src/runtime/benches/resident_ekf.rs"
        original_read_text = CHECKER.read_text

        def read_text(path):
            text = original_read_text(path)
            return text + '\nconst RETIRED: &str = "mech-legacy-atomic";\n' if path == benchmark else text

        with mock.patch.object(CHECKER, "read_text", side_effect=read_text):
            errors = CHECKER.static_contract_errors()
        self.assertTrue(
            any("legacy-atomic benchmark lane was restored" in error for error in errors)
        )

    def test_valid_b0_report_passes(self):
        self.assertEqual(CHECKER.report_contract_errors(valid_report()), [])

    def test_valid_b1_report_requires_resident_kernel_lanes(self):
        self.assertEqual(CHECKER.report_contract_errors(valid_b1_report()), [])

    def test_valid_b2_report_recomputes_complete_turn_decision(self):
        self.assertEqual(CHECKER.report_contract_errors(valid_b2_report()), [])

    def test_d1_artifact_lanes_extend_the_b2_lane_set(self):
        report = valid_b2_report()
        report["d1_decision"] = {}
        report["lanes"].extend(
            lane(name, instances, retained_history, next_epoch)
            for name, instances, retained_history, next_epoch in CHECKER.D1_ARTIFACT_LANE_KEYS
        )
        errors = CHECKER.report_contract_errors(report)
        self.assertFalse(any("unexpected lanes" in error for error in errors))

    def test_b2_descendant_refresh_requires_and_accepts_exact_commit(self):
        report = valid_b2_report()
        report["git_branch"] = "feat/core-semantic-foundations"
        self.assertIn(
            "B2 descendant refresh evidence requires an exact expected commit",
            CHECKER.report_contract_errors(report),
        )
        with mock.patch.object(CHECKER, "commit_descends_from", return_value=True):
            self.assertEqual(
                CHECKER.report_contract_errors(report, report["git_commit"]),
                [],
            )

    def test_b2_descendant_refresh_rejects_unrelated_commit(self):
        report = valid_b2_report()
        report["git_branch"] = "feat/core-semantic-foundations"
        with mock.patch.object(CHECKER, "commit_descends_from", return_value=False):
            errors = CHECKER.report_contract_errors(report, report["git_commit"])
        self.assertTrue(any("does not descend" in error for error in errors))

    def test_b2_frozen_historical_branch_does_not_require_ancestry_lookup(self):
        report = valid_b2_report()
        with mock.patch.object(CHECKER, "commit_descends_from") as ancestry:
            self.assertEqual(CHECKER.report_contract_errors(report), [])
        ancestry.assert_not_called()

    def test_commit_ancestry_distinguishes_descendant_unrelated_and_missing(self):
        for return_code, expected in ((0, True), (1, False), (128, False)):
            with self.subTest(return_code=return_code):
                completed = mock.Mock(returncode=return_code)
                with mock.patch.object(
                    CHECKER.subprocess, "run", return_value=completed
                ) as run:
                    self.assertEqual(
                        CHECKER.commit_descends_from("a" * 40),
                        expected,
                    )
                run.assert_called_once_with(
                    [
                        "git",
                        "merge-base",
                        "--is-ancestor",
                        CHECKER.B2_EVIDENCE_FLOOR,
                        "a" * 40,
                    ],
                    cwd=CHECKER.ROOT,
                    check=False,
                    stdout=CHECKER.subprocess.DEVNULL,
                    stderr=CHECKER.subprocess.DEVNULL,
                )

    def test_b2_rejects_forged_decision_metric(self):
        report = valid_b2_report()
        report["b2_decision"]["raw_epoch_ratio"] = 0.1
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("raw_epoch_ratio" in error for error in errors))

    def test_b2_rejects_history_iteration(self):
        report = valid_b2_report()
        result = next(
            lane
            for lane in report["lanes"]
            if lane["lane"] == "mech-resident-turn"
            and lane["retained_history"] == 100_000
        )
        result["structural"]["ledger_records_inspected"] = 100_000
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("ledger_records_inspected" in error for error in errors))

    def test_b2_history_independence_uses_five_percent_ceiling(self):
        report = valid_b2_report()
        result = next(
            lane
            for lane in report["lanes"]
            if lane["lane"] == "mech-resident-turn"
            and lane["retained_history"] == 1_000
        )
        result["timing"]["median_ns_per_turn"] = 26.5
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("hard gates" in error for error in errors))

    def test_b2_raw_epoch_requires_equivalent_record_evidence(self):
        report = valid_b2_report()
        result = next(
            lane
            for lane in report["lanes"]
            if lane["lane"] == "rust-epoch" and lane["instances"] == 8
        )
        result["structural"]["record_append_count"] = 0
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("raw epoch 8 reports wrong record_append_count" in error for error in errors))

    def test_unknown_phase_is_rejected(self):
        report = valid_report()
        report["phase"] = "B1-resident-kernel-typo"
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("unsupported Gate B report phase" in error for error in errors))

    def test_empty_and_unknown_future_phases_are_rejected(self):
        for phase in ("", "B2-dirty-scheduler"):
            report = valid_report()
            report["phase"] = phase
            errors = CHECKER.report_contract_errors(report)
            self.assertTrue(any("unsupported Gate B report phase" in error for error in errors))

    def test_b1_progression_is_recomputed_from_primary_medians(self):
        report = valid_b1_report()
        report["b1_progression"]["resident_kernel_vs_raw_epoch"] = 0.5
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("resident_kernel_vs_raw_epoch" in error for error in errors))

    def test_scaled_resident_requires_one_batch_publication(self):
        report = valid_b1_report()
        result = next(
            lane
            for lane in report["lanes"]
            if lane["lane"] == "mech-resident-kernel" and lane["instances"] == 64
        )
        result["structural"]["publication_store_count"] = 64
        errors = CHECKER.report_contract_errors(report)
        self.assertTrue(any("does not use one publication store" in error for error in errors))

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
