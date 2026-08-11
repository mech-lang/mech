from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_d2_contract", ROOT / "scripts/generate-d2-contract.py"
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class D2ContractGeneratorTests(unittest.TestCase):
    def projections(self):
        return {name: json.loads(path.read_text()) for name, path in GENERATOR.OUTPUTS.items()}

    def test_repository_contains_all_six_deterministic_projections(self):
        self.assertEqual(set(GENERATOR.OUTPUTS), {
            "profile", "artifact", "layout", "execution", "ekf", "reconfiguration"
        })
        for path in GENERATOR.OUTPUTS.values():
            self.assertTrue(path.exists())
            self.assertEqual(path.read_bytes(), GENERATOR.render(json.loads(path.read_text())))

    def test_projection_freezes_the_general_executor_boundaries(self):
        projections = self.projections()
        self.assertEqual(projections["artifact"]["legacy_opaque_contracts"], 0)
        self.assertEqual(projections["artifact"]["unclassified_nodes"], 0)
        self.assertEqual(projections["layout"]["candidate_bytes"], 480)
        self.assertEqual(projections["layout"]["candidate_seed_bytes"], 480)
        self.assertFalse(projections["layout"]["fixed_width_node_mask"])
        self.assertTrue(projections["execution"]["dirty_propagation"])
        self.assertEqual(projections["execution"]["energy_drift_quantization"], 1.0e-8)
        self.assertEqual(projections["execution"]["final_state_quantization"], 1.0e-10)
        self.assertEqual(
            projections["execution"]["trajectory_sha256_by_platform"],
            GENERATOR.FROZEN_NBODY_TRAJECTORIES,
        )
        self.assertEqual(projections["execution"]["steady_state_allocations"], 0)
        self.assertEqual(projections["profile"]["integrity_default"], "Checked")
        self.assertEqual(
            projections["profile"]["integrity_modes"], ["Checked", "Unchecked"]
        )


if __name__ == "__main__":
    unittest.main()
