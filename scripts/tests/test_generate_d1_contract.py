import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_d1_contract", ROOT / "scripts/generate-d1-contract.py"
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class D1ContractGeneratorTests(unittest.TestCase):
    def projections(self):
        return {
            name: json.loads(path.read_text())
            for name, path in GENERATOR.OUTPUTS.items()
        }

    def test_repository_projections_have_pinned_hashes(self):
        for name, path in GENERATOR.OUTPUTS.items():
            self.assertEqual(
                hashlib.sha256(path.read_bytes()).hexdigest(),
                GENERATOR.EXPECTED_SHA256[name],
            )

    def test_identity_fields_fail_closed(self):
        projections = self.projections()
        artifact = projections["artifact"]
        activation = projections["activation"]
        self.assertEqual(artifact["program_revision"], activation["program_revision"])
        self.assertTrue(artifact["source_bytecode_revision_equal"])
        self.assertEqual(artifact["legacy_opaque_contracts"], 0)
        self.assertEqual(artifact["unclassified_nodes"], 0)

    def test_execution_projection_freezes_the_complete_turn(self):
        execution = self.projections()["execution"]
        self.assertEqual(execution["turns"], 4096)
        self.assertNotIn("steady_state_allocations", execution)
        self.assertEqual(execution["candidate_written_bytes"], 96)
        self.assertEqual(execution["publication_store_count"], 1)
        self.assertEqual(execution["commit_runtime_calls"], 0)
        self.assertEqual(execution["legacy_journal_captures"], 0)
        self.assertTrue(execution["source_bytecode_trajectory_equal"])
        self.assertTrue(execution["gate_b_control_trajectory_equal"])
        self.assertTrue(execution["abort_preserves_published_epoch"])
        self.assertEqual(execution["ordinary_ekf_vertical_slice"], "complete")
        self.assertEqual(execution["admitted_artifacts"], 1)
        self.assertEqual(execution["migrated_state_slots"], 2)
        self.assertEqual(execution["global_d_targets_implemented"], 0)
        self.assertEqual(execution["legacy_targets_removed"], 0)
        self.assertEqual(execution["legacy_occurrences_migrated"], 0)


if __name__ == "__main__":
    unittest.main()
