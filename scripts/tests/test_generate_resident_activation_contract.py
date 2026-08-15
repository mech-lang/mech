from __future__ import annotations

import copy
import importlib.util
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/generate-resident-activation-contract.py"
SPEC = importlib.util.spec_from_file_location("generate_resident_activation_contract", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


class ResidentActivationGeneratorTests(unittest.TestCase):
    def test_repository_generated_contract_is_current(self) -> None:
        self.assertEqual(GENERATOR.validate(ROOT), [])

    def test_phase_d_projection_is_mechanical_and_sorted(self) -> None:
        projection = GENERATOR.build_migration_projection(ROOT)
        self.assertEqual(projection["target_count"], 2)
        self.assertEqual(projection["occurrence_count"], 491)
        self.assertEqual(
            [target["id"] for target in projection["targets"]],
            ["mutable-reference-runtime-storage", "uninitialized-storage"],
        )
        self.assertTrue(all(not target["implemented"] for target in projection["targets"]))
        self.assertTrue(all(not target["legacy_removed"] for target in projection["targets"]))

    def test_unlisted_ekf_operation_is_rejected(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        failures = GENERATOR.validate_source(source + "\nextra := ekf/unknown(state)\n", workload)
        self.assertTrue(any("unlisted EKF operation" in failure for failure in failures))

    def test_missing_integrity_definition_is_rejected(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        source = source.replace("finite-candidate! :=", "finite-candidate :=", 1)
        failures = GENERATOR.validate_source(source, workload)
        self.assertTrue(any("three integrity definitions" in failure for failure in failures))

    def test_source_digest_cannot_be_reblessed_from_changed_bytes(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_bytes()
        changed = source + b"\n"
        reblessed = GENERATOR.sha256_bytes(changed)
        failures = GENERATOR.validate_source_digest(changed, reblessed, reblessed + "\n")
        self.assertTrue(any("frozen D0 SHA-256" in failure for failure in failures))

    def test_node_change_detection_is_frozen_per_operation(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        workload["operations"][0]["change_detection"] = "ExactScalar"
        failures = GENERATOR.validate_source(source, workload)
        self.assertTrue(any("exact frozen 18-operation semantics" in failure for failure in failures))

    def test_predicate_change_detection_is_frozen_per_operation(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        workload["operations"][15]["change_detection"] = "KernelReported"
        failures = GENERATOR.validate_source(source, workload)
        self.assertTrue(any("exact frozen 18-operation semantics" in failure for failure in failures))

    def test_operation_alias_construction_and_interaction_are_frozen(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        mutations = (
            ("alias", "MayAlias"),
            ("construction", {"kind": "Build", "shape": "Declared"}),
            ("interaction", "Effect"),
        )
        for field, value in mutations:
            with self.subTest(field=field):
                changed = copy.deepcopy(workload)
                changed["operations"][4][field] = value
                failures = GENERATOR.validate_source(source, changed)
                self.assertTrue(
                    any("exact frozen 18-operation semantics" in failure for failure in failures)
                )

    def test_operation_order_is_frozen(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        workload["operations"][0], workload["operations"][1] = (
            workload["operations"][1],
            workload["operations"][0],
        )
        failures = GENERATOR.validate_source(source, workload)
        self.assertTrue(any("exact frozen 18-operation semantics" in failure for failure in failures))

    def test_integrity_constraint_ordinal_is_frozen(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        workload["integrity_constraints"][0]["predicate_operation_ordinal"] = 16
        failures = GENERATOR.validate_source(source, workload)
        self.assertTrue(any("integrity_constraints" in failure for failure in failures))

    def test_predicate_output_and_zero_output_assertion_are_distinct(self) -> None:
        operation = GENERATOR.EXPECTED_OPERATIONS[15]
        constraint = GENERATOR.EXPECTED_INTEGRITY_CONSTRAINTS[0]
        self.assertEqual(operation["output_schema"], "bool")
        self.assertEqual(operation["change_detection"], "ExactScalar")
        self.assertEqual(constraint["operation"], "integrity/assert")
        self.assertEqual(constraint["input_count"], 1)
        self.assertEqual(constraint["output_count"], 0)

    def test_finiteness_constraint_covers_state_and_covariance(self) -> None:
        source = (ROOT / "tests/architecture/resident-activation/ekf-source-v1.mec").read_text()
        workload = json.loads(
            (ROOT / "tests/architecture/resident-activation/ekf-workload-v1.json").read_text()
        )
        changed = source.replace(
            "finite-candidate! := ekf/candidate-finite(corrected-state,\n  symmetrized-covariance)",
            "finite-candidate! := ekf/candidate-finite(corrected-state)",
            1,
        )
        failures = GENERATOR.validate_source(changed, workload)
        self.assertTrue(any("both corrected state" in failure for failure in failures))

    def test_every_object_schema_is_closed(self) -> None:
        for name in (
            "d0-boundary-schema.json",
            "d0-migration-projection-schema.json",
            "ekf-workload-v1-schema.json",
        ):
            schema = json.loads(
                (ROOT / "tests/architecture/resident-activation" / name).read_text()
            )
            pending = [schema]
            while pending:
                value = pending.pop()
                if isinstance(value, dict):
                    if value.get("type") == "object":
                        self.assertIs(value.get("additionalProperties"), False, name)
                    pending.extend(value.values())
                elif isinstance(value, list):
                    pending.extend(value)


if __name__ == "__main__":
    unittest.main()
