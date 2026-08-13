from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-gate-d-contract.py"
SPEC = importlib.util.spec_from_file_location("check_gate_d_contract", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


def valid_d3_report():
    return {
        "schema_version": 1,
        "gate": "D",
        "phase": "D3-resident-external",
        "thresholds": copy.deepcopy(CHECKER.D3_THRESHOLDS),
        "hard_gates": {name: True for name in CHECKER.D3_HARD_GATES},
        "decision": "Pass",
    }


class GateDContractTests(unittest.TestCase):
    def test_exact_d3_hard_gate_schema_passes(self):
        errors, decision = CHECKER.validate_d3_contract(valid_d3_report())
        self.assertEqual(errors, [])
        self.assertEqual(decision, "Pass")

    def test_missing_d3_hard_gate_is_rejected(self):
        report = valid_d3_report()
        report["hard_gates"].pop("replay_exact")
        report["decision"] = "Fail"
        errors, decision = CHECKER.validate_d3_contract(report)
        self.assertIn("Gate D3 hard-gate names changed", errors)
        self.assertEqual(decision, "Fail")

    def test_non_boolean_d3_hard_gate_is_rejected(self):
        report = valid_d3_report()
        report["hard_gates"]["replay_exact"] = 1
        report["decision"] = "Fail"
        errors, decision = CHECKER.validate_d3_contract(report)
        self.assertIn("Gate D3 hard-gate values must be booleans", errors)
        self.assertEqual(decision, "Fail")

    def test_d2_shaped_report_cannot_claim_the_d3_pointer(self):
        report = valid_d3_report()
        report["phase"] = "D2-resident-nbody"
        errors, decision = CHECKER.validate_d3_contract(report)
        self.assertIn("Gate D3 report phase changed", errors)
        self.assertEqual(decision, "Pass")

    def test_d3_schema_is_frozen(self):
        report = valid_d3_report()
        report["schema_version"] = 2
        errors, decision = CHECKER.validate_d3_contract(report)
        self.assertIn("Gate D3 report schema changed", errors)
        self.assertEqual(decision, "Pass")

    def test_d2_report_is_rejected_by_authoritative_d3_invocation(self):
        report = json.loads(
            (CHECKER.ROOT / "benchmarks/runtime/gate-d/d2-resident-nbody.json").read_text(
                encoding="utf-8"
            )
        )
        report["semantic_commit"] = "d2-shaped-at-d3-path"
        report_bytes = (json.dumps(report, sort_keys=True) + "\n").encode()
        with tempfile.TemporaryDirectory(dir=CHECKER.ROOT) as temporary:
            directory = Path(temporary)
            report_path = directory / "d3-resident-external.json"
            pointer_path = directory / "gate-d3-evidence.json"
            report_path.write_bytes(report_bytes)
            pointer_path.write_text(
                json.dumps(
                    {
                        "semantic_commit": report["semantic_commit"],
                        "evidence_path": report_path.relative_to(CHECKER.ROOT).as_posix(),
                        "evidence_sha256": hashlib.sha256(report_bytes).hexdigest(),
                    }
                ),
                encoding="utf-8",
            )
            result = CHECKER.main(
                [
                    "--report",
                    str(report_path),
                    "--pointer",
                    str(pointer_path),
                    "--expected-phase",
                    "D3-resident-external",
                ]
            )
        self.assertEqual(result, 1)


if __name__ == "__main__":
    unittest.main()
