"""Cover the existing math surface partition in native-link-only builds."""

import importlib.util
import json
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
    def test_sharded_catalog_union_is_the_extended_rust_contract(self):
        entries = [
            {"runtime_factory_id": "0000000000000002", "runtime_factory_name": "Two"},
            {"runtime_factory_id": "0000000000000001", "runtime_factory_name": "One"},
        ]
        surface_digest = LINKAGE.catalog_surface_digest(entries)
        report = {
            "complete_catalog": {
                "entry_count": 2 + len(LINKAGE.ADDITIVE_RUNTIME_FACTORIES),
                "runtime_surface_digest": surface_digest,
                "retained_entry_count": 2,
                "retained_runtime_surface_digest": surface_digest,
            }
        }
        contract = (
            f"const EXPECTED_EXTENDED_RUNTIME_FACTORIES: usize = {report['complete_catalog']['entry_count']};\n"
            "const EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST: &str =\n"
            f'    "{surface_digest}";\n'
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile_contracts.rs"
            path.write_text(contract)
            with patch.object(LINKAGE, "STDLIB_PROFILE_CONTRACT", path):
                LINKAGE.verify_extended_runtime_contract(report)
                report["complete_catalog"]["retained_runtime_surface_digest"] = "0" * 64
                with self.assertRaisesRegex(
                    LINKAGE.ContractError, "digest diverges"
                ):
                    LINKAGE.verify_extended_runtime_contract(report)

    def test_namespaced_custom_contracts_retain_fail_closed_linkage_validation(self):
        entry = {
            "name": "Example", "id_hex": "0000000000000001",
            "runtime_signature": "unary(any, any)", "signature_cargo_features": [],
            "package": "mech-engine", "crate_name": "mech_engine",
            "installer_path": "mech_engine::__mech_native::install_example",
            "cargo_features": ["native-link", "runtime"],
            "contract_kind": "number/parse-u64",
            "output_alias_policy": "disallow_input_alias",
        }
        known = {"mech-engine": {"native-link", "runtime"}}
        for kind in ["no_matrix", "number/parse-u64", "number/format",
                     "sequence/length", "map/histogram", "map/keys", "map/values",
                     "sequence/sort-indices", "text/lines", "text/join"]:
            with self.subTest(kind=kind):
                result = LINKAGE.validate_catalog([{**entry, "contract_kind": kind}], "test", known)
                self.assertEqual(result[0]["contract_kind"], kind)
        for kind in [None, "", "/number", "number/", "number//parse", "number/../parse",
                     "number/parse value", "unknown", "unchecked", "infer_from_name", "best_effort"]:
            with self.subTest(kind=kind):
                with self.assertRaisesRegex(LINKAGE.ContractError, "contract kind"):
                    LINKAGE.validate_catalog([{**entry, "contract_kind": kind}], "test", known)
        for changes, error in [
            ({"installer_path": "not a Rust path"}, "invalid linkage metadata"),
            ({"cargo_features": ["runtime"]}, "omits required native features"),
            ({"output_alias_policy": "allow_input_alias"}, "outside register families"),
            ({"signature_cargo_features": ["f64"]}, "omits signature-derived"),
        ]:
            with self.subTest(changes=changes):
                with self.assertRaisesRegex(LINKAGE.ContractError, error):
                    LINKAGE.validate_catalog([{**entry, **changes}], "test", known)

    def test_additive_surface_preserves_every_retained_factory(self):
        retained = {"runtime_factory_name": "Old", "runtime_factory_id": "01"}
        additions = [
            {"runtime_factory_name": name, "runtime_factory_id": f"{index + 2:02x}"}
            for index, name in enumerate(sorted(LINKAGE.ADDITIVE_RUNTIME_FACTORIES))
        ]
        entries = [retained, *additions]
        self.assertEqual(LINKAGE.retained_runtime_entries(entries), [retained])
        for malformed in [entries[:-1], [*entries, additions[0]]]:
            with self.assertRaisesRegex(LINKAGE.ContractError, "every exact additive factory once"):
                LINKAGE.retained_runtime_entries(malformed)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frozen.json"
            raw = json.dumps({"runtime_factories": [
                {"name": entry["runtime_factory_name"], "id_hex": entry["runtime_factory_id"]}
                for entry in entries
            ]}).encode()
            path.write_bytes(raw)
            with patch.object(LINKAGE, "FROZEN_SURFACE", path), \
                 patch.object(LINKAGE, "EXPECTED_FULL_COUNT", len(entries)), \
                 patch.object(LINKAGE, "EXPECTED_FULL_SURFACE_SHA256", LINKAGE.sha256(raw).hexdigest()), \
                 patch.object(LINKAGE, "EXPECTED_RETAINED_FULL_RUNTIME_SURFACE_DIGEST",
                              LINKAGE.digest([retained])):
                LINKAGE.verify_full_surface(entries)
                for malformed in [
                    additions,
                    [{**retained, "runtime_factory_id": "02"}, *additions],
                    [*entries, retained],
                    [*entries, {"runtime_factory_name": "Unreviewed", "runtime_factory_id": "ff"}],
                ]:
                    with self.assertRaisesRegex(LINKAGE.ContractError, "frozen full surface"):
                        LINKAGE.verify_full_surface(malformed)

    def test_malformed_extended_rust_contract_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile_contracts.rs"
            path.write_text("const SOMETHING_ELSE: usize = 2;\n")
            with patch.object(LINKAGE, "STDLIB_PROFILE_CONTRACT", path):
                with self.assertRaisesRegex(LINKAGE.ContractError, "missing or malformed"):
                    LINKAGE.frozen_extended_runtime_contract()

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
