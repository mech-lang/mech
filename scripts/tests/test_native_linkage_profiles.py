"""Cover the existing math surface partition in native-link-only builds."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "native_linkage", ROOT / "scripts/check-native-linkage-coverage.py"
)
LINKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LINKAGE)


class NativeLinkageProfileTests(unittest.TestCase):
    def test_math_shards_preserve_all_value_features_without_planning(self):
        profiles = LINKAGE.owner_native_link_profiles("mech-math")
        self.assertEqual(len(profiles), len(LINKAGE.MATH_SURFACE_SHARDS))
        selected = [set(profile.split()) for profile in profiles]
        features = LINKAGE.tomllib.loads(
            (ROOT / "machines/math/Cargo.toml").read_text()
        )["features"]
        self.assertTrue(set(features["full_values"]).issubset(set().union(*selected)))
        for profile in selected:
            self.assertTrue({"native-link", "full_operations", "f32", "f64"} <= profile)
            self.assertFalse({"native-plan", "source", "compiler", "full_runtime"} & profile)
        rational = selected[LINKAGE.MATH_SURFACE_SHARDS.index("extended-math-shard-rational")]
        self.assertTrue({"i32", "r64"} <= rational)

    def test_omitting_a_scalar_shard_is_rejected(self):
        shards = tuple(s for s in LINKAGE.MATH_SURFACE_SHARDS if "unsigned-small" not in s)
        with patch.object(LINKAGE, "MATH_SURFACE_SHARDS", shards):
            with self.assertRaisesRegex(LINKAGE.ContractError, "omit features"):
                LINKAGE.owner_native_link_profiles("mech-math")

    def test_unknown_shard_is_rejected(self):
        with patch.object(LINKAGE, "MATH_SURFACE_SHARDS", ("missing-shard",)):
            with self.assertRaisesRegex(LINKAGE.ContractError, "missing math surface"):
                LINKAGE.owner_native_link_profiles("mech-math")

    def test_accidental_source_role_is_rejected(self):
        source = LINKAGE.FIXTURE_MANIFEST.read_text().replace(
            '"mech-math/full_operations",',
            '"mech-math/full_operations", "mech-math/source",',
        )
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            manifest.write_text(source)
            with patch.object(LINKAGE, "FIXTURE_MANIFEST", manifest):
                with self.assertRaisesRegex(LINKAGE.ContractError, "leaks source/planning"):
                    LINKAGE.owner_native_link_profiles("mech-math")

    def test_every_math_shard_is_executed_and_failures_propagate(self):
        expected = LINKAGE.owner_native_link_profiles("mech-math")
        with patch.object(LINKAGE, "run") as run:
            LINKAGE.verify_owner_native_link_profiles(["mech-math"])
        self.assertEqual([call.args[0][-1] for call in run.call_args_list], expected)
        with patch.object(LINKAGE, "run", side_effect=LINKAGE.ContractError("compile failed")):
            with self.assertRaisesRegex(LINKAGE.ContractError, "compile failed"):
                LINKAGE.verify_owner_native_link_profiles(["mech-math"])

    def test_other_owners_and_matrix_minimal_profiles_are_unchanged(self):
        owners = [p for p in LINKAGE.OWNERS if p != "mech-math"]
        with patch.object(LINKAGE, "run") as run:
            LINKAGE.verify_owner_native_link_profiles(owners)
        expected = []
        for package in owners:
            manifest, _, profile = LINKAGE.OWNERS[package]
            profiles = [f"{profile} native-link"]
            if package == "mech-matrix":
                profiles.extend(LINKAGE.MATRIX_MINIMAL_NATIVE_LINK_PROFILES)
            expected.extend((str(manifest), profile) for profile in profiles)
        actual = [(c.args[0][4], c.args[0][-1]) for c in run.call_args_list]
        self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main()
