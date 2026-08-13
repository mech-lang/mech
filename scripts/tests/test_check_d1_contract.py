from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("check_d1_contract", ROOT / "scripts/check-d1-contract.py")
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class D1ContractTests(unittest.TestCase):
    def projections(self):
        projections, errors = CHECKER.projection_files(ROOT)
        self.assertEqual(errors, [])
        return projections

    def test_repository_contract_passes_without_evidence_topology(self):
        self.assertEqual(CHECKER.run(ROOT, contract_only=True), [])

    def test_projection_rejects_opaque_or_unclassified_nodes(self):
        for field in ("legacy_opaque_contracts", "unclassified_nodes"):
            projections = copy.deepcopy(self.projections())
            projections["artifact"][field] = 1
            failures = CHECKER.projection_errors(projections)
            self.assertTrue(any(field in failure for failure in failures))

    def test_projection_rejects_migration_overclaim(self):
        projections = copy.deepcopy(self.projections())
        projections["execution"]["global_d_targets_implemented"] = 1
        failures = CHECKER.projection_errors(projections)
        self.assertTrue(any("global_d_targets_implemented" in failure for failure in failures))

    def test_changed_path_allowlist_is_fail_closed(self):
        failures = CHECKER.changed_path_errors(["src/runtime/src/runtime/execution/mod.rs"])
        self.assertTrue(any("outside the exact allowlist" in failure for failure in failures))

    def test_branch_check_accepts_ci_detached_head(self):
        result = subprocess.CompletedProcess(["git", "branch", "--show-current"], 0, "", "")
        self.assertEqual(CHECKER.branch_name_errors(result), [])

    def test_branch_check_rejects_an_attached_wrong_branch(self):
        result = subprocess.CompletedProcess(
            ["git", "branch", "--show-current"], 0, "feat/unrelated\n", ""
        )
        failures = CHECKER.branch_name_errors(result)
        self.assertEqual(
            failures,
            [
                "D1 branch must remain feat/resident-ekf-program-artifact-path; "
                "found feat/unrelated"
            ],
        )

    def test_branch_check_fails_closed_when_git_cannot_inspect_head(self):
        result = subprocess.CompletedProcess(["git", "branch", "--show-current"], 128, "", "fatal")
        self.assertEqual(CHECKER.branch_name_errors(result), ["unable to determine the D1 branch name"])

    def test_public_artifact_and_hot_turn_boundaries_pass(self):
        self.assertEqual(CHECKER.source_contract_errors(ROOT), [])


if __name__ == "__main__":
    unittest.main()
