from __future__ import annotations

import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_d2_contract", ROOT / "scripts/generate-d2-contract.py"
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)
import d2_historical_evidence as HISTORICAL


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

    def test_historical_executor_materializes_then_uses_locked_offline_dependencies(self):
        manifest = Path("/tmp/historical-d2/Cargo.toml")
        fetch, run = HISTORICAL.historical_cargo_commands(
            manifest, "--probe", release=True
        )
        self.assertEqual(
            fetch[:4],
            ["cargo", "+nightly-2026-03-03", "fetch", "--manifest-path"],
        )
        self.assertEqual(fetch[-1], str(manifest))
        self.assertIn("--locked", run)
        self.assertIn("--offline", run)
        self.assertIn("--release", run)
        self.assertEqual(run[-2:], ["--", "--probe"])

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

    def test_fixture_facts_require_the_live_historical_executor_trajectory(self):
        current = "D2_PROJECTION platform=aarch64-macos trajectory=abc legacy_exact=true\n"
        historical = "D2_PROJECTION platform=aarch64-macos trajectory=abc legacy_exact=true\n"
        with mock.patch.object(
            GENERATOR.subprocess,
            "run",
            return_value=SimpleNamespace(
                returncode=0,
                stdout=current,
                stderr="",
            ),
        ), mock.patch.object(
            GENERATOR,
            "run_historical_d2_fixture",
            return_value=historical,
        ):
            self.assertEqual(GENERATOR.fixture_facts()["legacy_exact"], "true")

    def test_fixture_facts_reject_historical_executor_drift(self):
        current = "D2_PROJECTION platform=aarch64-macos trajectory=current\n"
        historical = "D2_PROJECTION platform=aarch64-macos trajectory=historical\n"
        with mock.patch.object(
            GENERATOR.subprocess,
            "run",
            return_value=SimpleNamespace(
                returncode=0,
                stdout=current,
                stderr="",
            ),
        ), mock.patch.object(
            GENERATOR,
            "run_historical_d2_fixture",
            return_value=historical,
        ):
            with self.assertRaisesRegex(RuntimeError, "live historical D2 executor"):
                GENERATOR.fixture_facts()


if __name__ == "__main__":
    unittest.main()
