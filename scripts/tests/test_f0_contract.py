#!/usr/bin/env python3
"""Behavioral and mutation tests for the compact F0 contract."""

from __future__ import annotations

import copy
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))


def module(name: str, relative: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / relative)
    value = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = value
    spec.loader.exec_module(value)
    return value


CONTRACT = module("compact_f0_contract", "f0_contract.py")
SHARED = module("compact_f0_shared", "f0_evidence.py")
RUNNER = module("compact_f0_runner", "run-f0-qualification.py")
GATE_D = module("compact_gate_d_runner", "run-gate-d-benchmarks.py")
INSTALLER = module(
    "compact_f0_installer", "install-f0-measurement-toolchain.py"
)


class CompactF0ContractTests(unittest.TestCase):
    def manifest(self) -> dict:
        product = SHARED.load_json(SHARED.PRODUCT_TREE_MANIFEST)
        return {
            "schema_version": 1,
            "protocol_version": SHARED.PROTOCOL_VERSION,
            "product_subject": {
                "commit": product["baseline_commit"],
                "tree": product["baseline_tree"],
                "guard": "tests/architecture/qualification/f0-product-tree.json",
            },
            "protocol": {
                "commit": None,
                "tree": None,
                "contract": "scripts/f0_contract.py",
            },
            "environment": {
                "toolchain_lock": (
                    "tests/architecture/qualification/f0-measurement-toolchain.json"
                ),
                "toolchain_lock_sha256": SHARED.sha256_file(
                    SHARED.TOOLCHAIN_MANIFEST
                ),
                "qualification_environment_id": None,
            },
            "replication_rule": copy.deepcopy(CONTRACT.REPLICATION_RULE),
            "evidence": None,
            "closeout": None,
        }

    def test_protocol_manifest_passes_without_pretending_evidence_exists(self):
        self.assertEqual(CONTRACT.validate(self.manifest()), [])

    def test_changed_product_subject_fails(self):
        manifest = self.manifest()
        manifest["product_subject"]["tree"] = "0" * 40
        self.assertTrue(
            any("product tree differs" in error for error in CONTRACT.validate(manifest))
        )

    def test_changed_toolchain_bytes_fail(self):
        manifest = self.manifest()
        manifest["environment"]["toolchain_lock_sha256"] = "0" * 64
        self.assertTrue(
            any("toolchain bytes changed" in error for error in CONTRACT.validate(manifest))
        )

    def test_changed_report_bytes_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / "report.json"
            report.write_text("{}\n", encoding="utf-8")
            reference = {"path": "report.json", "sha256": SHARED.sha256_file(report)}
            report.write_text('{"changed":true}\n', encoding="utf-8")
            errors = []
            CONTRACT.reference(reference, "F0 B2 report", errors, root)
        self.assertIn("F0 B2 report bytes changed", errors)

    def test_changed_provenance_fails(self):
        left = {"provenance": {"chain_id": "chain-1"}}
        right = {"provenance": {"chain_id": "chain-2"}}
        self.assertTrue(SHARED.same_provenance(left, right, ("chain_id",)))

    def test_closeout_cannot_precede_evidence(self):
        manifest = self.manifest()
        manifest["closeout"] = {}
        self.assertTrue(
            any("cannot precede" in error for error in CONTRACT.validate(manifest))
        )

    def ledger(self) -> tuple[dict, dict, dict]:
        manifest = self.manifest()
        manifest["protocol"].update({"commit": "1" * 40, "tree": "2" * 40})
        manifest["environment"]["qualification_environment_id"] = "3" * 64
        evidence = {
            "generation_commit": "4" * 40,
            "generation_tree": "5" * 40,
        }
        chains = [
            {
                "chain_id": chain,
                "status": "Pass",
                "steps": [
                    {"phase": phase, "returncode": 0} for phase in CONTRACT.PHASES
                ],
            }
            for chain in CONTRACT.RECORDED_CHAINS
        ]
        ledger = {
            "status": "Pass",
            "protocol_version": SHARED.PROTOCOL_VERSION,
            "runtime_subject_commit": manifest["product_subject"]["commit"],
            "runtime_subject_tree": manifest["product_subject"]["tree"],
            "qualification_protocol_commit": manifest["protocol"]["commit"],
            "evidence_generation_commit": evidence["generation_commit"],
            "qualification_environment_id": manifest["environment"][
                "qualification_environment_id"
            ],
            "preconditioning": {"status": "Pass", "commands": []},
            "chains": chains,
        }
        return manifest, evidence, ledger

    def test_missing_recorded_chain_fails(self):
        manifest, evidence, ledger = self.ledger()
        ledger["chains"].pop()
        self.assertTrue(
            any(
                "recorded-chain order changed" in error
                for error in CONTRACT.ledger_errors(ledger, evidence, manifest)
            )
        )

    def test_failed_chain_cannot_be_hidden(self):
        manifest, evidence, ledger = self.ledger()
        ledger["chains"][1]["status"] = "Fail"
        self.assertTrue(
            any(
                "chain-2 did not pass" in error
                for error in CONTRACT.ledger_errors(ledger, evidence, manifest)
            )
        )

    def test_preconditioning_cannot_become_a_fourth_evidence_chain(self):
        manifest, evidence, ledger = self.ledger()
        ledger["preconditioning"]["chain_id"] = "preconditioning"
        self.assertTrue(
            any(
                "became an evidence chain" in error
                for error in CONTRACT.ledger_errors(ledger, evidence, manifest)
            )
        )

    def test_d3_must_authenticate_exact_d2_bytes_and_provenance(self):
        provenance = {
            "runtime_subject_tree": "1" * 40,
            "qualification_environment_id": "2" * 64,
            "protocol_version": SHARED.PROTOCOL_VERSION,
            "chain_id": "chain-1",
        }
        d2_ref = {"sha256": "3" * 64}
        d2 = {"decision": "Fail", "provenance": provenance}
        d3 = {
            "d2_authentication": {
                "evidence_sha256": d2_ref["sha256"],
                "decision": "Fail",
                "qualification_decision": "Pass",
                **provenance,
            }
        }
        self.assertEqual(CONTRACT.d3_binding_errors(d2_ref, d2, d3), [])
        d3["d2_authentication"]["evidence_sha256"] = "4" * 64
        self.assertTrue(CONTRACT.d3_binding_errors(d2_ref, d2, d3))

    def test_absolute_d2_performance_findings_are_advisory_only(self):
        report = SHARED.load_json(
            ROOT / "benchmarks/runtime/gate-d/d2-resident-nbody.json"
        )
        qualification, findings = CONTRACT.d2_qualification(report)
        self.assertEqual(qualification, "Pass")
        self.assertEqual(
            findings,
            {
                "nbody": ["legacy_gap_closure", "resident_raw_ratio"],
                "ekf": ["complete_d1_ratio", "kernel_d1_ratio"],
            },
        )
        report["nbody"]["hard_gates"]["source_bytecode_ratio"] = False
        report["ekf"]["hard_gates"]["source_bytecode_ratio"] = False
        qualification, findings = CONTRACT.d2_qualification(report)
        self.assertEqual(qualification, "Pass")
        self.assertIn("source_bytecode_ratio", findings["nbody"])
        self.assertIn("source_bytecode_ratio", findings["ekf"])
        report["nbody"]["hard_gates"]["history_independent"] = False
        self.assertEqual(CONTRACT.d2_qualification(report)[0], "Fail")

    def test_d3_regression_timing_is_advisory_but_effect_contracts_block(self):
        gates = {name: True for name in CONTRACT.D3_HARD_GATES}
        gates["d2_pure_regression"] = False
        report = {"hard_gates": gates}
        self.assertEqual(CONTRACT.d3_qualification(report)[0], "Pass")
        gates["replay_exact"] = False
        self.assertEqual(CONTRACT.d3_qualification(report)[0], "Fail")

    def test_gate_b_speed_targets_are_advisory_but_boundedness_blocks(self):
        report = SHARED.load_json(
            ROOT / "benchmarks/runtime/gate-b/b2-resident-turn.json"
        )
        for section, advisory in CONTRACT.GATE_B_ADVISORY_PERFORMANCE_GATES.items():
            for gate in advisory:
                report[section]["hard_gates"][gate] = False
        qualification, findings = CONTRACT.gate_b_qualification(report)
        self.assertEqual(qualification, "Pass")
        self.assertEqual(
            findings,
            {
                section: sorted(advisory)
                for section, advisory in CONTRACT.GATE_B_ADVISORY_PERFORMANCE_GATES.items()
            },
        )
        report["b2_decision"]["hard_gates"]["history_independent"] = False
        self.assertEqual(CONTRACT.gate_b_qualification(report)[0], "Fail")

    def test_uncontrolled_compiler_environment_fails(self):
        self.assertEqual(
            SHARED.uncontrolled_build_environment(
                {"CARGO_INCREMENTAL": "0"}, {"CARGO_INCREMENTAL": "0"}
            ),
            {},
        )
        self.assertEqual(
            SHARED.uncontrolled_build_environment(
                {"CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "shadow"}, {}
            ),
            {"CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "shadow"},
        )

    def test_preconditioning_builds_only_and_produces_no_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def run_logged(arguments, _environment, log):
                log.write_text("prepared\n", encoding="utf-8")
                self.assertIn("build" if "gate-d3" not in log.name else "test", arguments)
                return 0

            with mock.patch.object(RUNNER, "run_logged", side_effect=run_logged):
                record = RUNNER.run_preconditioning(root, {}, "nightly-test")
        self.assertEqual(record["status"], "Pass")
        self.assertNotIn("chain_id", record)
        self.assertNotIn("reports", record)
        self.assertEqual(
            [command["name"] for command in record["commands"]],
            ["gate-b", "gate-d2", "gate-d3"],
        )
        gate_d3 = record["commands"][2]["arguments"]
        self.assertIn(
            "source_default,resident-routing-source,runtime_bench_gate_d3",
            gate_d3,
        )

    def test_measurement_lock_contains_only_measurement_inputs(self):
        lock = SHARED.load_json(
            ROOT
            / "tests/architecture/qualification/f0-measurement-toolchain.json"
        )
        self.assertEqual(
            set(lock),
            {
                "schema_version",
                "protocol_version",
                "canonical_platform",
                "cargo_lock_sha256",
                "rust",
                "python",
                "numpy",
                "thread_environment",
                "compiler_environment",
                "cargo_home_configuration",
                "measurement_conditions",
            },
        )
        rendered = str(lock).lower()
        for unused in ("chrome", "chromedriver", "node", "npm", "wasm-pack"):
            self.assertNotIn(unused, rendered)

    def test_d2_uses_only_the_historical_legacy_lane(self):
        fresh = "GATE_D_SAMPLE lane=nbody-raw-rust sample=0 turns=4096 elapsed_ns=1"
        historical = "\n".join(
            (
                "GATE_D_SAMPLE lane=nbody-raw-rust sample=0 turns=4096 elapsed_ns=99",
                "GATE_D_SAMPLE lane=nbody-legacy-mech sample=0 turns=4096 elapsed_ns=42",
            )
        )
        measured = GATE_D.d2_measurement_raw(fresh, historical)
        self.assertIn("elapsed_ns=1", measured)
        self.assertIn("elapsed_ns=42", measured)
        self.assertNotIn("elapsed_ns=99", measured)

    def test_measurement_power_settings_select_only_the_active_source(self):
        output = """
Battery Power:
 lowpowermode 1
AC Power:
 lowpowermode 0
"""
        self.assertEqual(
            INSTALLER.active_power_settings(output, "AC Power"),
            {"lowpowermode": "0"},
        )

    def test_controlled_runner_accepts_only_fixed_branch_workflows(self):
        for workflow in ("ci-full.yml", "f0-controlled.yml"):
            self.assertTrue(
                RUNNER.trusted_workflow_ref(
                    f"mech-lang/mech/.github/workflows/{workflow}@"
                    "refs/heads/qualification/f0-final-evidence"
                )
            )
        self.assertFalse(
            RUNNER.trusted_workflow_ref(
                "mech-lang/mech/.github/workflows/f0-controlled.yml@refs/heads/attacker"
            )
        )
        self.assertFalse(RUNNER.trusted_workflow_ref(None))


if __name__ == "__main__":
    unittest.main()
