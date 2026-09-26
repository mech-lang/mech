"""Cover native profile partitions and the explicit Boolean-reduction addition."""

import importlib.util
import copy
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
    @staticmethod
    def entry(index, name):
        return {
            **copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY),
            "runtime_factory_id": f"{index:016x}",
            "runtime_factory_name": name,
            "installer_path": f"mech_logic::__mech_native::install_{name.lower()}",
            "cargo_features": ["bool", "native-link", "not", "runtime"],
            "contract_kind": "same_shape",
            "runtime_signature": "RuntimeFunctionSignature { output: Bool, inputs: Unary(Bool) }",
        }

    @staticmethod
    def report(entries):
        return {
            "complete_catalog": {
                "entry_count": len(entries),
                "runtime_surface_digest": LINKAGE.catalog_surface_digest(entries),
            },
            "entries": LINKAGE.grouped(entries),
        }

    def test_sharded_catalog_union_is_the_extended_rust_contract(self):
        entries = [self.entry(2, "Two"), self.entry(1, "One")]
        surface_digest = LINKAGE.catalog_surface_digest(entries)
        report = self.report([*entries, copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY)])
        contract = (
            "const EXPECTED_EXTENDED_RUNTIME_FACTORIES: usize = 2;\n"
            "const EXPECTED_EXTENDED_RUNTIME_SURFACE_DIGEST: &str =\n"
            f'    "{surface_digest}";\n'
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile_contracts.rs"
            path.write_text(contract)
            with patch.object(LINKAGE, "STDLIB_PROFILE_CONTRACT", path):
                LINKAGE.verify_extended_runtime_contract(report)
                report["complete_catalog"]["runtime_surface_digest"] = "0" * 64
                with self.assertRaisesRegex(
                    LINKAGE.ContractError, "digest diverges"
                ):
                    LINKAGE.verify_extended_runtime_contract(report)

    def test_full_surface_accepts_only_the_exact_addition(self):
        legacy = [self.entry(1, "One"), self.entry(2, "Two")]
        frozen = {"runtime_factories": [
            {"id_hex": entry["runtime_factory_id"], "name": entry["runtime_factory_name"]}
            for entry in legacy
        ]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "runtime-factory-surface.json"
            path.write_text(json.dumps(frozen))
            with patch.object(LINKAGE, "FROZEN_SURFACE", path), \
                 patch.object(LINKAGE, "EXPECTED_FULL_COUNT", len(legacy)), \
                 patch.object(LINKAGE, "EXPECTED_FULL_SURFACE_SHA256", LINKAGE.sha256(path.read_bytes()).hexdigest()):
                addition = copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY)
                LINKAGE.verify_full_surface([*legacy, addition])
                for invalid in [
                    legacy,
                    [legacy[0], addition],
                    [*legacy, addition, self.entry(3, "Unknown")],
                    [*legacy, addition, addition],
                    [legacy[0], self.entry(2, "Renamed"), addition],
                ]:
                    with self.subTest(invalid=invalid), self.assertRaises(LINKAGE.ContractError):
                        LINKAGE.verify_full_surface(invalid)

    def test_addition_metadata_and_id_are_exact(self):
        entry = copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY)
        LINKAGE.validate_logic_all_entry(entry)
        mutations = {
            "runtime_factory_id": "0000000000000009",
            "runtime_factory_name": "logic/any",
            "runtime_signature": "RuntimeFunctionSignature { output: Bool, inputs: Unary(F64) }",
            "signature_cargo_features": ["bool", "f64"],
            "package": "mech-math",
            "crate_name": "mech_math",
            "installer_path": "mech_logic::__mech_native::other",
            "cargo_features": ["bool", "native-link", "runtime"],
            "contract_kind": "no_matrix",
            "output_alias_policy": "allow_input_alias",
        }
        for field, value in mutations.items():
            with self.subTest(field=field), self.assertRaisesRegex(LINKAGE.ContractError, "logic/all"):
                LINKAGE.historical_entries([{**entry, field: value}])
        raw = {
            "name": entry["runtime_factory_name"],
            "id_hex": entry["runtime_factory_id"],
            **{key: value for key, value in entry.items() if key not in {"runtime_factory_name", "runtime_factory_id"}},
        }
        known = {"mech-logic": {"all", "bool", "native-link", "runtime"}}
        self.assertEqual(LINKAGE.validate_catalog([raw], "addition", known), [entry])
        with self.assertRaisesRegex(LINKAGE.ContractError, "logic/all"):
            LINKAGE.validate_catalog([{**raw, "id_hex": "0000000000000009"}], "addition", known)

    def test_extended_contract_rejects_unknown_removed_or_changed_entries(self):
        legacy = [self.entry(1, "One"), self.entry(2, "Two")]
        addition = copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY)
        with patch.object(LINKAGE, "frozen_extended_runtime_contract", return_value=(2, LINKAGE.catalog_surface_digest(legacy))):
            LINKAGE.verify_extended_runtime_contract(self.report([*legacy, addition]))
            for invalid in [
                legacy,
                [legacy[0], addition],
                [*legacy, self.entry(3, "Unknown"), addition],
                [legacy[0], self.entry(2, "Changed"), addition],
                [*legacy, {**addition, "runtime_factory_id": "0000000000000009"}],
            ]:
                with self.subTest(invalid=invalid), self.assertRaises(LINKAGE.ContractError):
                    LINKAGE.verify_extended_runtime_contract(self.report(invalid))

    def test_reports_keep_actual_surface_and_label_historical_compatibility(self):
        one, two = self.entry(1, "One"), self.entry(2, "Two")
        addition = copy.deepcopy(LINKAGE.LOGIC_ALL_ENTRY)
        report = LINKAGE.assemble_report([one, addition], [[one, two, addition]])
        baseline = LINKAGE.report_summary(LINKAGE.assemble_surface_report([one], [one, two]))
        self.assertEqual(report["complete_catalog"]["entry_count"], 3)
        self.assertEqual(report["complete_catalog"]["runtime_surface_digest"], LINKAGE.catalog_surface_digest([one, two, addition]))
        self.assertEqual(report["historical_baseline"]["coverage_summary"], baseline)
        self.assertEqual(LINKAGE.verify_committed_summary(report, baseline), "historical baseline plus the explicit logic/all addition")
        self.assertEqual(LINKAGE.verify_committed_summary(report, LINKAGE.report_summary(report)), "current catalog")
        changed = {**one, "installer_path": "mech_logic::__mech_native::changed"}
        changed_report = LINKAGE.assemble_report([changed, addition], [[changed, two, addition]])
        with self.assertRaisesRegex(LINKAGE.ContractError, "stale"):
            LINKAGE.verify_committed_summary(changed_report, baseline)

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
