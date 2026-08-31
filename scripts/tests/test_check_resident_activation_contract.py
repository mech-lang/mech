from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check-resident-activation-contract.py"
FIXTURES = ROOT / "scripts/tests/fixtures/resident-activation"
SPEC = importlib.util.spec_from_file_location("check_resident_activation_contract", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class ResidentActivationCheckerTests(unittest.TestCase):
    def fixture(self, name: str) -> dict:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        source = FIXTURES / name / "fixture.json"
        target = root / name / "fixture.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
        return json.loads(target.read_text(encoding="utf-8"))

    def workload(self) -> tuple[str, dict]:
        directory = ROOT / "tests/architecture/resident-activation"
        return (
            (directory / "ekf-source-v1.mec").read_text(encoding="utf-8"),
            json.loads((directory / "ekf-workload-v1.json").read_text(encoding="utf-8")),
        )

    def test_repository_state_passes(self) -> None:
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_production_changes_fail(self) -> None:
        fixture = self.fixture("production-change")
        boundary = json.loads(
            (ROOT / "tests/architecture/resident-activation/d0-boundary.json").read_text()
        )
        failures = CHECKER.validate_changed_paths(
            fixture["changed_paths"], list(CHECKER.D0_ALLOWED_CHANGES)
        )
        self.assertTrue(any("outside the exact D0 allowlist" in failure for failure in failures))

    def test_boundary_cannot_authorize_a_production_file(self) -> None:
        boundary = json.loads(
            (ROOT / "tests/architecture/resident-activation/d0-boundary.json").read_text()
        )
        mutations = (
            ("base_commit", "f" * 40),
            ("pr_base_commit", "e" * 40),
            ("allowed_changes", boundary["allowed_changes"] + ["src/engine/src/"]),
        )
        for field, value in mutations:
            with self.subTest(field=field):
                changed = copy.deepcopy(boundary)
                changed[field] = value
                failures = CHECKER.validate_boundary_policy(changed)
                self.assertTrue(any(field in failure for failure in failures))
                path_failures = CHECKER.validate_changed_paths(
                    ["src/engine/src/program/instance.rs"],
                    list(CHECKER.D0_ALLOWED_CHANGES),
                )
                self.assertTrue(any("outside the exact D0 allowlist" in failure for failure in path_failures))

    def test_publication_sequence_is_exact(self) -> None:
        boundary = json.loads(
            (ROOT / "tests/architecture/resident-activation/d0-boundary.json").read_text()
        )
        boundary["publication_contract"]["candidate_executes_before_receipt_preparation"] = False
        failures = CHECKER.validate_boundary_policy(boundary)
        self.assertTrue(any("publication_contract" in failure for failure in failures))

    def test_publication_step_order_cannot_be_rearranged(self) -> None:
        boundary = json.loads(
            (ROOT / "tests/architecture/resident-activation/d0-boundary.json").read_text()
        )
        steps = boundary["publication_contract"]["ordered_steps"]
        steps[3], steps[4] = steps[4], steps[3]
        failures = CHECKER.validate_boundary_policy(boundary)
        self.assertTrue(any("publication_contract" in failure for failure in failures))

    def test_new_legacy_dependency_fails(self) -> None:
        fixture = self.fixture("new-legacy-dependency")
        failures = CHECKER.validate_new_legacy_dependencies(
            {fixture["path"]: fixture["current"]},
            {fixture["path"]: fixture["baseline"]},
        )
        self.assertTrue(any("LegacyValue" in failure for failure in failures))

    def test_transaction_dependency_fails(self) -> None:
        fixture = self.fixture("new-transaction-dependency")
        failures = CHECKER.validate_new_legacy_dependencies(
            {fixture["path"]: fixture["current"]},
            {fixture["path"]: fixture["baseline"]},
        )
        self.assertTrue(
            any("RuntimeExecutionTransaction" in failure for failure in failures)
        )

    def test_pointer_identity_fails(self) -> None:
        fixture = self.fixture("pointer-identity")
        sources = {
            fixture["path"]: fixture["source"],
            "src/engine/src/resident/arena.rs": (
                "#[cfg(test)]\nfn buffer_addresses(values: &[u8]) {\n"
                " let _ = values.as_ptr(); let _ = values.as_ptr();\n"
                " let _ = values.as_ptr(); let _ = values.as_ptr();\n}\n"
            ),
        }
        failures = CHECKER.validate_pointer_identity(sources)
        self.assertTrue(any("kernel.rs" in failure and "as_ptr" in failure for failure in failures))

    def test_hot_turn_schema_lookup_fails(self) -> None:
        fixture = self.fixture("hot-turn-schema-lookup")
        failures = CHECKER.validate_hot_turn_boundary(
            {fixture["path"]: fixture["current"]},
            {fixture["path"]: fixture["baseline"]},
        )
        self.assertTrue(any("SchemaTable" in failure for failure in failures))

    def test_duplicate_artifact_authority_fails(self) -> None:
        fixture = self.fixture("duplicate-artifact")
        failures = CHECKER.validate_artifact_authority(fixture["sources"])
        self.assertTrue(any("exactly" in failure for failure in failures))

    def test_unlisted_ekf_operation_fails(self) -> None:
        fixture = self.fixture("unlisted-operation")
        source, workload = self.workload()
        failures = CHECKER.validate_source_contract(
            source + fixture["append_source"], workload, ROOT
        )
        self.assertTrue(any("unlisted EKF operation" in failure for failure in failures))

    def test_missing_integrity_constraint_fails(self) -> None:
        fixture = self.fixture("missing-integrity")
        source, workload = self.workload()
        changed = source.replace(
            fixture["replace_from"], fixture["replace_to"], 1
        )
        failures = CHECKER.validate_source_contract(changed, workload, ROOT)
        self.assertTrue(any("three integrity definitions" in failure for failure in failures))

    def test_missing_permanent_activation_owner_fails(self) -> None:
        contract = json.loads(
            (
                ROOT
                / "tests/architecture/resident-activation/resident-activation-contract.json"
            ).read_text()
        )
        contract["activation_owners"][0]["path"] = "src/engine/src/artifact/missing.rs"
        failures = CHECKER.validate_activation_structure(ROOT, contract)
        self.assertTrue(any("required activation owner is missing" in failure for failure in failures))

    def test_migration_fields_are_rejected_from_permanent_contract(self) -> None:
        contract = json.loads(
            (
                ROOT
                / "tests/architecture/resident-activation/resident-activation-contract.json"
            ).read_text()
        )
        contract["semantic_targets"][0]["occurrences"] = []
        failures = CHECKER.validate_activation_structure(ROOT, contract)
        self.assertTrue(any("migration field occurrences" in failure for failure in failures))

    def test_stale_gate_b_evidence_fails(self) -> None:
        fixture = self.fixture("gate-b-drift")
        failures = CHECKER.validate_gate_b_expected(fixture["expected_commit"])
        self.assertTrue(any("Gate B expected implementation drifted" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
