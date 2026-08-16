import copy
import contextlib
import hashlib
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CONTRACTS = ROOT / "tests/architecture/value-system"
SCRIPT = Path(__file__).resolve().parents[1] / "check-value-system-contract.py"
SPEC = importlib.util.spec_from_file_location("check_value_system_contract", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


VALUE_SOURCE = """
pub enum ValueKind {
    Any,
    Empty,
    Reference(Box<ValueKind>),
    Kind,
}

pub enum Value {
    MutableReference(MutableReference),
    Empty,
    MatrixValue(Matrix<Value>),
    EmptyKind(ValueKind),
    Kind(ValueKind),
}
"""

KIND_SOURCE = """
pub enum Kind {
    Any,
    Empty,
    Reference(Box<Kind>),
    Kind(Box<Kind>),
}
"""

NODES_SOURCE = """
pub enum Kind { Any }
pub struct KindAnnotation { pub kind: Kind }
pub struct Var { pub kind: Option<KindAnnotation> }
pub struct Binding { pub kind: Option<KindAnnotation> }
pub struct Field { pub kind: Option<KindAnnotation> }
pub struct KindDefine { pub kind: KindAnnotation }
pub struct EnumVariant { pub value: Option<KindAnnotation> }
pub struct Fsm { pub kind: Option<KindAnnotation> }
pub struct FunctionArgument { pub kind: KindAnnotation }
pub struct FunctionDefine { pub input: Vec<FunctionArgument>, pub output: Vec<FunctionArgument> }
pub struct FsmSpecification { pub input: Vec<Var>, pub output: Option<KindAnnotation> }
"""

ARGUMENT_SOURCE = """
pub enum FunctionArgumentRole { Input }
pub enum FunctionMatrixRepresentation { Dynamic }
pub struct FunctionMatrixDescriptor;
"""

SIGNATURE_SOURCE = """
pub enum FunctionMatrixElement { F64 }
pub enum FunctionMatrixStoragePattern { AnyStorage }
pub enum FunctionValueRepresentation { F64 }
pub enum RuntimeFunctionInputs { Nullary }
pub struct RuntimeFunctionSignature;
pub trait FunctionRuntimeType {}
pub enum NativeValueFeature { F64 }
"""

LIVE_SOURCE = """
use crate::kind::Kind;

fn classified_uses() {
    let _ = Value::Empty;
    let _ = Value::Empty;
    let _ = Value::Empty;
    let _ = Value::Empty;
    let _ = Value::Empty;
    let _ = Value::Empty;
    let _ = Value::MatrixValue;
    let _ = Value::MatrixValue;
    let _ = Value::MatrixValue;
    let _ = ValueKind::Empty;
    let _ = Kind::Empty;
    let _ = Value::Kind;
    let _ = Kind::Reference;
    let _ = ValueKind::Any;
    let _ = Value::EmptyKind;
    let _ = Value::MutableReference;
}
"""


def target(identifier, category, gate, representation=None):
    result = {
        "id": identifier,
        "semantic_category": category,
        "representation": representation or identifier,
        "implementation_gate": gate,
        "key_semantics": "not-keyable",
        "runtime_storage": "not-applicable",
    }
    result["status"] = CHECKER.expected_target_status(result)
    return result


class ReviewedContractsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inventory = CHECKER.load_json(CONTRACTS / "current-inventory.json")
        cls.inventory_schema = CHECKER.load_json(CONTRACTS / "current-inventory-schema.json")
        cls.migration = CHECKER.load_json(CONTRACTS / "migration.json")
        cls.migration_schema = CHECKER.load_json(CONTRACTS / "migration-schema.json")
        cls.baseline = CHECKER.load_json(CONTRACTS / "legacy-growth-baseline.json")
        cls.baseline_schema = CHECKER.load_json(CONTRACTS / "legacy-growth-baseline-schema.json")
        cls.canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        cls.canonical_schema = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1-schema.json")
        cls.canonical_vectors = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1-vectors.json")
        cls.canonical_vectors_schema = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1-vectors-schema.json")
        cls.frozen_targets = CHECKER.load_json(CONTRACTS / "frozen-semantic-targets-v1.json")
        cls.frozen_targets_schema = CHECKER.load_json(CONTRACTS / "frozen-semantic-targets-v1-schema.json")

    def test_all_reviewed_documents_match_their_schemas(self):
        for payload, schema in (
            (self.inventory, self.inventory_schema),
            (self.migration, self.migration_schema),
            (self.baseline, self.baseline_schema),
            (self.canonical, self.canonical_schema),
            (self.canonical_vectors, self.canonical_vectors_schema),
            (self.frozen_targets, self.frozen_targets_schema),
        ):
            self.assertEqual(CHECKER.schema_errors(payload, schema), [])

    def test_every_enumerated_rust_source_has_one_effective_disposition(self):
        self.assertEqual(
            CHECKER.source_disposition_failures(
                self.inventory,
                CONTRACTS / "current-inventory.json",
            ),
            [],
        )

        missing = copy.deepcopy(self.inventory)
        missing["audited_rust_files"].remove("src/core/src/value.rs")
        self.assertEqual(
            {failure.contract_id for failure in CHECKER.source_disposition_failures(
                missing,
                CONTRACTS / "current-inventory.json",
            )},
            {"C0-AUDITED-SOURCE-SET"},
        )

        overlapping = copy.deepcopy(self.inventory)
        cargo_path = overlapping["auxiliary_cargo_fixtures"][0]["targets"][0][
            "reachable_rust_files"
        ][0]
        overlapping["auxiliary_rust_fixtures"][0]["reachable_rust_files"].append(
            cargo_path
        )
        self.assertEqual(
            {failure.contract_id for failure in CHECKER.source_disposition_failures(
                overlapping,
                CONTRACTS / "current-inventory.json",
            )},
            {"C0-AUDITED-SOURCE-SET"},
        )

    def test_all_three_enums_are_covered_exactly_once(self):
        for enum_name, field in CHECKER.ENUM_FIELDS.items():
            expected = {
                row["name"] for row in self.inventory["enums"][enum_name]["variants"]
            }
            actual = [
                variant
                for family in self.migration["families"]
                for variant in family["current"][field]
            ]
            self.assertEqual(set(actual), expected)
            self.assertEqual(len(actual), len(set(actual)))

    def test_every_occurrence_is_classified_exactly_once(self):
        expected = {
            (row["enum"], row["variant"], row["path"], row["line"], row["column"])
            for row in self.inventory["variant_uses"]
        }
        actual = [
            (row["enum"], row["variant"], row["path"], site["line"], site["column"])
            for row in self.migration["use_classifications"]
            for site in row["sites"]
        ]
        self.assertEqual(set(actual), expected)
        self.assertEqual(len(actual), len(set(actual)))

    def test_empty_has_exact_six_nonempty_semantic_targets(self):
        expected = {
            "source-empty-expression",
            "option-absence",
            "execution-no-result",
            "uninitialized-storage",
            "unspecified-extent",
            "generic-dispatch",
        }
        actual = CHECKER.classified_targets(self.migration, "LegacyValue", "Empty")
        self.assertEqual(set(actual), expected)
        for identifier in expected:
            self.assertGreater(actual.count(identifier), 0)

    def test_matrix_value_has_exact_three_live_targets_and_zero_rejections(self):
        expected = {
            "matrix-construction-ir",
            "homogeneous-matrix-snapshot",
            "legacy-matrix-value-adapter",
        }
        actual = CHECKER.classified_targets(self.migration, "LegacyValue", "MatrixValue")
        self.assertEqual(set(actual), expected)
        for identifier in expected:
            self.assertGreater(actual.count(identifier), 0)
        self.assertNotIn("heterogeneous-matrix-rejected", actual)

    def test_matrix_value_fallback_and_adapter_sites_are_exact(self):
        sites = {
            (row["path"], site["line"], site["column"]): row["target"]
            for row in self.migration["use_classifications"]
            if row["enum"] == "LegacyValue" and row["variant"] == "MatrixValue"
            for site in row["sites"]
        }
        self.assertEqual(
            sites[("src/engine/src/structures.rs", 742, 19)],
            "homogeneous-matrix-snapshot",
        )
        self.assertEqual(
            sites[("src/runtime/src/runtime/program/external/value_adapter.rs", 45, 13)],
            "legacy-matrix-value-adapter",
        )
        self.assertEqual(
            sum(target == "legacy-matrix-value-adapter" for target in sites.values()),
            9,
        )

    def test_targets_are_unambiguous_and_have_frozen_status(self):
        targets = [
            item
            for family in self.migration["families"]
            for item in family["targets"]
        ]
        self.assertEqual(len(targets), 69)
        for family in self.migration["families"]:
            for item in family["targets"]:
                self.assertIsNone(CHECKER.AMBIGUOUS_TARGET.search(json.dumps(item)))
                self.assertEqual(
                    item["status"], CHECKER.expected_target_status(item)
                )

    def test_type_contract_source_inventory_is_exact(self):
        self.assertEqual(
            self.inventory["type_contract_sources"],
            CHECKER.GENERATOR.type_contract_sources(ROOT),
        )

    def test_kind_and_runtime_signature_sources_are_separated(self):
        groups = self.inventory["type_contract_sources"]
        expression = {row["symbol"]: row for row in groups["kind_expression_sources"]}
        schemes = {row["symbol"]: row for row in groups["kind_scheme_sources"]}
        runtime = {row["symbol"]: row for row in groups["runtime_representation_sources"]}
        self.assertEqual(expression["KindAnnotation"]["target"], "KindExpr")
        self.assertEqual(schemes["FunctionDefine.input"]["target"], "KindScheme")
        self.assertEqual(schemes["FunctionDefine.output"]["target"], "KindScheme")
        self.assertEqual(
            runtime["FunctionMatrixStoragePattern"]["target"],
            "RuntimeRepresentationSignature and native-lowering metadata",
        )

    def test_runtime_representation_cannot_be_reclassified_as_kind_scheme(self):
        live = copy.deepcopy(self.inventory)
        live["type_contract_sources"]["runtime_representation_sources"][0][
            "target"
        ] = "KindScheme"
        failures = CHECKER.type_contract_source_failures(
            ROOT, live, CONTRACTS / "current-inventory.json"
        )
        self.assertIn(
            "C0-KIND-SCHEME-SEPARATION",
            {item.contract_id for item in failures},
        )

    def test_canonical_encoding_constants_are_exact(self):
        self.assertEqual(
            CHECKER.canonical_encoding_failures(
                self.canonical, CONTRACTS / "canonical-encoding-v1.json"
            ),
            [],
        )
        self.assertEqual(
            CHECKER.canonical_vector_failures(
                self.canonical_vectors,
                CONTRACTS / "canonical-encoding-v1-vectors.json",
            ),
            [],
        )


class TargetApplicabilityTests(unittest.TestCase):
    def setUp(self):
        self.migration = CHECKER.load_json(CONTRACTS / "migration.json")
        self.path = CONTRACTS / "migration.json"

    def failures(self, migration=None):
        return CHECKER.target_applicability_failures(
            migration or self.migration, self.path
        )

    def target(self, migration, identifier):
        return next(
            target
            for family in migration["families"]
            for target in family["targets"]
            if target["id"] == identifier
        )

    def classification(self, migration, enum_name, variant):
        return next(
            row
            for row in migration["use_classifications"]
            if row["enum"] == enum_name and row["variant"] == variant
        )

    def test_reviewed_applicability_is_exact(self):
        self.assertEqual(self.failures(), [])
        targets, _owners = CHECKER.target_index(self.migration)
        self.assertIn(
            ("LegacyValue", "F64"),
            {
                (row["enum"], variant)
                for row in targets["floating-point-snapshot"]["applies_to"]
                for variant in row["variants"]
            },
        )
        self.assertIn(
            ("ValueKind", "F64"),
            {
                (row["enum"], variant)
                for row in targets["floating-point-schema"]["applies_to"]
                for variant in row["variants"]
            },
        )
        self.assertIn(
            ("Kind", "Reference"),
            {
                (row["enum"], variant)
                for row in targets["reference-binding-contract"]["applies_to"]
                for variant in row["variants"]
            },
        )

    def test_value_snapshot_cannot_select_schema_target(self):
        migration = copy.deepcopy(self.migration)
        self.classification(migration, "LegacyValue", "F64")["target"] = "floating-point-schema"
        self.assertTrue(self.failures(migration))

    def test_value_kind_schema_cannot_select_snapshot_target(self):
        migration = copy.deepcopy(self.migration)
        self.classification(migration, "ValueKind", "F64")["target"] = "floating-point-snapshot"
        self.assertTrue(self.failures(migration))

    def test_mutable_reference_cannot_select_binding_target(self):
        migration = copy.deepcopy(self.migration)
        self.classification(migration, "LegacyValue", "MutableReference")[
            "target"
        ] = "reference-binding-contract"
        self.assertTrue(self.failures(migration))

    def test_target_cannot_apply_outside_owning_family(self):
        migration = copy.deepcopy(self.migration)
        self.target(migration, "floating-point-snapshot")["applies_to"] = [
            {"enum": "LegacyValue", "variants": ["String"]}
        ]
        self.assertTrue(self.failures(migration))

    def test_target_cannot_have_empty_applicability(self):
        migration = copy.deepcopy(self.migration)
        self.target(migration, "floating-point-snapshot")["applies_to"] = []
        self.assertTrue(self.failures(migration))

    def test_every_family_variant_needs_an_applicable_target(self):
        migration = copy.deepcopy(self.migration)
        family = next(
            family
            for family in migration["families"]
            if family["id"] == "scalar/floating-point"
        )
        for target in family["targets"]:
            for row in target["applies_to"]:
                row["variants"] = [variant for variant in row["variants"] if variant != "F64"]
        self.assertTrue(self.failures(migration))


class FrozenTargetProjectionTests(unittest.TestCase):
    def setUp(self):
        self.migration = CHECKER.load_json(CONTRACTS / "migration.json")
        self.frozen = CHECKER.load_json(CONTRACTS / "frozen-semantic-targets-v1.json")

    def failures(self, migration):
        return CHECKER.frozen_target_failures(
            migration,
            self.frozen,
            CONTRACTS / "migration.json",
            CONTRACTS / "frozen-semantic-targets-v1.json",
        )

    def test_reviewed_projection_matches(self):
        self.assertEqual(self.failures(self.migration), [])

    def test_every_normative_field_is_frozen_for_representative_targets(self):
        mutations = {
            "semantic_category": "execution-control",
            "representation": "ChangedRepresentation",
            "implementation_gate": "changed-gate",
            "key_semantics": "changed-key-semantics",
            "runtime_storage": "changed-storage",
            "applies_to": [{"enum": "LegacyValue", "variants": ["Bool"]}],
        }
        target_ids = (
            "option-absence",
            "homogeneous-matrix-snapshot",
            "heterogeneous-matrix-rejected",
            "reified-type-snapshot",
            "reference-binding-contract",
        )
        for target_id in target_ids:
            for field, replacement in mutations.items():
                with self.subTest(target=target_id, field=field):
                    migration = copy.deepcopy(self.migration)
                    target = next(
                        target
                        for family in migration["families"]
                        for target in family["targets"]
                        if target["id"] == target_id
                    )
                    target[field] = copy.deepcopy(replacement)
                    self.assertIn(
                        "C0-FROZEN-TARGET-DRIFT",
                        {item.contract_id for item in self.failures(migration)},
                    )


class FrozenOccurrenceTargetTests(unittest.TestCase):
    def setUp(self):
        self.migration = CHECKER.load_json(CONTRACTS / "migration.json")
        self.frozen = CHECKER.load_json(
            CONTRACTS / "frozen-semantic-targets-v1.json"
        )

    def failures(self, migration):
        return CHECKER.frozen_occurrence_target_failures(
            migration,
            self.frozen,
            CONTRACTS / "migration.json",
            CONTRACTS / "frozen-semantic-targets-v1.json",
        )

    def test_reviewed_occurrence_targets_match(self):
        self.assertEqual(self.failures(self.migration), [])

    def test_swapping_two_empty_occurrence_targets_fails(self):
        migration = copy.deepcopy(self.migration)
        first = next(
            row
            for row in migration["use_classifications"]
            if row["enum"] == "LegacyValue" and row["variant"] == "Empty"
        )
        second = next(
            row
            for row in migration["use_classifications"]
            if row["enum"] == "LegacyValue"
            and row["variant"] == "Empty"
            and row["target"] != first["target"]
        )
        first["target"], second["target"] = second["target"], first["target"]
        self.assertIn(
            "C0-FROZEN-OCCURRENCE-TARGET",
            {item.contract_id for item in self.failures(migration)},
        )


class ValueSystemContractFixtureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write(
            "Cargo.toml",
            '[workspace]\nmembers = ["src/core", "src/engine"]\nresolver = "2"\n',
        )
        self.write(
            "src/core/Cargo.toml",
            '[package]\nname = "core-fixture"\nversion = "0.0.0"\n',
        )
        self.write("src/core/src/value.rs", VALUE_SOURCE)
        self.write("src/core/src/kind.rs", KIND_SOURCE)
        self.write(
            "src/core/src/lib.rs",
            "mod value;\nmod kind;\npub mod program;\npub mod types;\npub use self::program::*;\n",
        )
        self.write(
            "src/core/src/program/mod.rs",
            "pub mod symbol_table;\npub use self::symbol_table::*;\n",
        )
        self.write(
            "src/core/src/program/symbol_table.rs",
            "pub type SymbolTableRef = Ref<SymbolTable>;\n",
        )
        self.write(
            "src/core/src/types/mod.rs",
            "pub type MutableReference = Ref<Value>;\npub type ValRef = Ref<Value>;\n",
        )
        self.write(
            "src/engine/Cargo.toml",
            '[package]\nname = "engine-fixture"\nversion = "0.0.0"\n',
        )
        self.write(
            "src/engine/src/lib.rs",
            "pub mod interpreter;\npub use crate::interpreter::*;\n",
        )
        self.write(
            "src/engine/src/interpreter/mod.rs",
            "pub type InterpreterRef = Ref<Box<Interpreter>>;\n",
        )
        self.write("src/core/src/live.rs", LIVE_SOURCE)
        self.write("src/core/src/nodes.rs", NODES_SOURCE)
        self.write("src/core/src/function/argument.rs", ARGUMENT_SOURCE)
        self.write("src/core/src/function/signature.rs", SIGNATURE_SOURCE)
        self.contract_root = self.root / "tests/architecture/value-system"
        self.contract_root.mkdir(parents=True)
        self.reference = "f" * 40
        baseline_inventory = CHECKER.GENERATOR.generate(self.root, self.reference)
        self.baseline = CHECKER.GENERATOR.legacy_baseline(
            baseline_inventory, self.reference
        )
        self.write(
            "src/engine/src/lib.rs",
            '#[cfg(feature = "semantic-compiler")]\nmod interpreter;\n',
        )
        self.write(
            "src/engine/src/interpreter/mod.rs",
            "pub(crate) type InterpreterRef = Ref<Box<Interpreter>>;\n",
        )
        self.inventory = CHECKER.GENERATOR.generate(self.root, self.reference)
        self.inventory_path = self.save("current-inventory.json", self.inventory)
        self.migration = self.make_migration()
        self.migration_path = self.save("migration.json", self.migration)
        self.baseline_path = self.save("legacy-growth-baseline.json", self.baseline)
        self.canonical_path = self.copy_contract("canonical-encoding-v1.json")
        self.inventory_schema_path = self.copy_contract("current-inventory-schema.json")
        self.migration_schema_path = self.copy_contract("migration-schema.json")
        self.baseline_schema_path = self.copy_contract("legacy-growth-baseline-schema.json")
        baseline_schema = CHECKER.load_json(self.baseline_schema_path)
        baseline_schema["properties"]["reference_commit"] = {"const": self.reference}
        self.save_path(self.baseline_schema_path, baseline_schema)
        self.canonical_schema_path = self.copy_contract("canonical-encoding-v1-schema.json")
        self.canonical_vectors_path = self.copy_contract("canonical-encoding-v1-vectors.json")
        self.canonical_vectors_schema_path = self.copy_contract("canonical-encoding-v1-vectors-schema.json")
        self.frozen_targets_schema_path = self.copy_contract("frozen-semantic-targets-v1-schema.json")
        self.frozen_targets = {
            "schema_version": 1,
            "targets": CHECKER.target_projection(self.migration),
            "occurrence_targets": CHECKER.occurrence_target_projection(
                self.migration
            ),
        }
        self.frozen_targets_path = self.save("frozen-semantic-targets-v1.json", self.frozen_targets)
        evidence_relative = "benchmarks/runtime/gate-b/b2-resident-turn.json"
        self.evidence_path = self.root / evidence_relative
        self.evidence_path.parent.mkdir(parents=True)
        self.evidence_path.write_bytes((ROOT / evidence_relative).read_bytes())
        self.gate_b = CHECKER.load_json(CONTRACTS / "gate-b-regression.json")
        self.gate_b["reference_commit"] = self.reference
        self.gate_b["evidence_sha256"] = hashlib.sha256(self.evidence_path.read_bytes()).hexdigest()
        self.gate_b_path = self.save("gate-b-regression.json", self.gate_b)

    def write(self, relative, source):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")
        return path

    def save_path(self, path, payload):
        path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return path

    def save(self, name, payload):
        return self.save_path(self.contract_root / name, payload)

    def copy_contract(self, name):
        path = self.contract_root / name
        path.write_bytes((CONTRACTS / name).read_bytes())
        return path

    def make_migration(self):
        families = [
            {
                "id": "legacy/reference",
                "current": {
                    "value_variants": ["MutableReference"],
                    "value_kind_variants": ["Reference"],
                    "kind_variants": ["Reference"],
                    "features": [],
                    "storage": ["MutableReference"],
                },
                "targets": [
                    target("mutable-reference-runtime-storage", "runtime-slot-state", "D"),
                    target("reference-binding-contract", "binding-contract", "C4"),
                ],
            },
            {
                "id": "legacy/empty-sentinel",
                "current": {
                    "value_variants": ["Empty"],
                    "value_kind_variants": ["Empty"],
                    "kind_variants": ["Empty"],
                    "features": [],
                    "storage": [],
                },
                "targets": [
                    target("source-empty-expression", "compiler-construction-ir", "C3"),
                    target("option-absence", "immutable-snapshot", "C2"),
                    target("execution-no-result", "execution-control", "C3"),
                    target("uninitialized-storage", "runtime-slot-state", "D"),
                    target("unspecified-extent", "compiler-shape-hole", "C1"),
                    target("generic-dispatch", "legacy-dispatch", "final-cutover"),
                    target("value-kind-hole", "kind-expression", "C1"),
                    target("kind-hole", "kind-expression", "C1"),
                ],
            },
            {
                "id": "matrix/value-backed-legacy",
                "current": {
                    "value_variants": ["MatrixValue"],
                    "value_kind_variants": [],
                    "kind_variants": [],
                    "features": [],
                    "storage": ["Matrix<Value>"],
                },
                "targets": [
                    target("matrix-construction-ir", "compiler-construction-ir", "C3"),
                    target("homogeneous-matrix-snapshot", "immutable-snapshot", "C2"),
                    target("heterogeneous-matrix-rejected", "rejected-legacy-form", "C2"),
                    target("legacy-matrix-value-adapter", "legacy-dispatch", "C2"),
                ],
            },
            {
                "id": "legacy/typed-empty",
                "current": {
                    "value_variants": ["EmptyKind"],
                    "value_kind_variants": [],
                    "kind_variants": [],
                    "features": [],
                    "storage": ["ValueKind"],
                },
                "targets": [target("legacy-typed-empty-adapter", "legacy-dispatch", "C2")],
            },
            {
                "id": "meta/reified-type",
                "current": {
                    "value_variants": ["Kind"],
                    "value_kind_variants": ["Kind"],
                    "kind_variants": ["Kind"],
                    "features": [],
                    "storage": ["ValueKind"],
                },
                "targets": [
                    target("reified-type-snapshot", "reified-type-snapshot", "C2"),
                    target("reified-type-schema", "schema", "C1"),
                    target("type-of-kind-expression", "kind-expression", "C1"),
                ],
            },
            {
                "id": "kind/wildcard",
                "current": {
                    "value_variants": [],
                    "value_kind_variants": ["Any"],
                    "kind_variants": ["Any"],
                    "features": [],
                    "storage": [],
                },
                "targets": [target("kind-wildcard", "kind-expression", "C1")],
            },
        ]
        empty_targets = iter(
            [
                "source-empty-expression",
                "option-absence",
                "execution-no-result",
                "uninitialized-storage",
                "unspecified-extent",
                "generic-dispatch",
            ]
        )
        matrix_targets = iter(
            [
                "matrix-construction-ir",
                "homogeneous-matrix-snapshot",
                "legacy-matrix-value-adapter",
            ]
        )
        defaults = {
            ("LegacyValue", "MutableReference"): "mutable-reference-runtime-storage",
            ("LegacyValue", "EmptyKind"): "legacy-typed-empty-adapter",
            ("LegacyValue", "Kind"): "reified-type-snapshot",
            ("ValueKind", "Empty"): "value-kind-hole",
            ("ValueKind", "Any"): "kind-wildcard",
            ("Kind", "Empty"): "kind-hole",
            ("Kind", "Reference"): "reference-binding-contract",
        }
        classifications = []
        for use in self.inventory["variant_uses"]:
            key = (use["enum"], use["variant"])
            if key == ("LegacyValue", "Empty"):
                destination = next(empty_targets)
            elif key == ("LegacyValue", "MatrixValue"):
                destination = next(matrix_targets)
            else:
                destination = defaults[key]
            classifications.append(
                {
                    "enum": use["enum"],
                    "variant": use["variant"],
                    "path": use["path"],
                    "sites": [{"line": use["line"], "column": use["column"]}],
                    "roles": ["semantic-payload"],
                    "target": destination,
                }
            )
        for family in families:
            members = family["current"]
            for item in family["targets"]:
                if item["id"] == "reference-binding-contract":
                    applicable = [
                        ("ValueKind", members["value_kind_variants"]),
                        ("Kind", members["kind_variants"]),
                    ]
                elif item["id"] == "kind-wildcard":
                    applicable = [
                        ("ValueKind", members["value_kind_variants"]),
                        ("Kind", members["kind_variants"]),
                    ]
                elif item["id"] == "value-kind-hole":
                    applicable = [("ValueKind", members["value_kind_variants"])]
                elif item["id"] == "kind-hole":
                    applicable = [("Kind", members["kind_variants"])]
                elif item["semantic_category"] == "schema":
                    applicable = [("ValueKind", members["value_kind_variants"])]
                elif item["semantic_category"] == "kind-expression":
                    applicable = [("Kind", members["kind_variants"])]
                else:
                    applicable = [("LegacyValue", members["value_variants"])]
                item["applies_to"] = [
                    {"enum": enum_name, "variants": list(variants)}
                    for enum_name, variants in applicable
                    if variants
                ]
        return {
            "schema_version": 4,
            "reference_commit": self.reference,
            "families": families,
            "authorized_high_risk_uses": [],
            "use_classifications": classifications,
        }

    def audit(self):
        return CHECKER.audit(
            self.root,
            self.inventory_path,
            self.migration_path,
            self.gate_b_path,
            self.inventory_schema_path,
            self.migration_schema_path,
            baseline_path=self.baseline_path,
            baseline_schema_path=self.baseline_schema_path,
            canonical_path=self.canonical_path,
            canonical_schema_path=self.canonical_schema_path,
            canonical_vectors_path=self.canonical_vectors_path,
            canonical_vectors_schema_path=self.canonical_vectors_schema_path,
            frozen_targets_path=self.frozen_targets_path,
            frozen_targets_schema_path=self.frozen_targets_schema_path,
            verify_reference=False,
            check_gate_a=False,
            check_c2_adapter=False,
            baseline_inventory=self.baseline,
        )

    @staticmethod
    def ids(failures):
        return {item.contract_id for item in failures}

    def test_valid_fixture_passes(self):
        self.assertEqual(self.audit(), [])

    def test_permanent_compatibility_aliases_are_exact_and_public(self):
        cases = (
            (
                "private",
                "src/core/src/program/symbol_table.rs",
                "type SymbolTableRef = Ref<SymbolTable>;\n",
            ),
        )
        for name, path, source in cases:
            with self.subTest(name=name):
                original = (self.root / path).read_text(encoding="utf-8")
                self.write(path, source)
                self.assertIn("C0-PUBLIC-COMPAT-ALIAS", self.ids(self.audit()))
                self.write(path, original)

    def test_retired_interpreter_alias_cannot_be_republished(self):
        self.write(
            "src/engine/src/lib.rs",
            "pub mod interpreter;\npub use crate::interpreter::*;\n",
        )
        self.write(
            "src/engine/src/interpreter/mod.rs",
            "pub type InterpreterRef = Ref<Interpreter>;\n",
        )
        self.assertIn("C0-PUBLIC-COMPAT-ALIAS", self.ids(self.audit()))

    def test_retired_interpreter_alias_publication_is_permitted(self):
        self.write(
            "src/engine/src/interpreter/mod.rs",
            "pub(crate) type InterpreterRef = Ref<Box<Interpreter>>;\n",
        )
        self.write(
            "src/engine/src/lib.rs",
            '#[cfg(feature = "semantic-compiler")]\nmod interpreter;\n',
        )
        self.assertNotIn("C0-PUBLIC-COMPAT-ALIAS", self.ids(self.audit()))

    def test_legacy_value_alias_removal_is_permitted(self):
        self.write(
            "src/core/src/types/mod.rs",
            "pub type ValRef = Ref<Value>;\n",
        )
        self.assertNotIn(
            "C0-IMMUTABLE-LEGACY-BASELINE", self.ids(self.audit())
        )

    def test_legacy_value_alias_target_drift_fails(self):
        self.write(
            "src/core/src/types/mod.rs",
            "pub type MutableReference = Ref<Other>;\n"
            "pub type ValRef = Ref<Value>;\n",
        )
        self.assertIn(
            "C0-IMMUTABLE-LEGACY-BASELINE", self.ids(self.audit())
        )

    def test_raw_approved_alias_reports_raw_spelling_and_target_drift(self):
        self.write(
            "src/core/src/types/mod.rs",
            "pub type MutableReference = Ref<Value>;\n"
            "pub type r#ValRef = Ref<Other>;\n",
        )
        identifiers = self.ids(self.audit())
        self.assertIn("C0-RAW-APPROVED-ALIAS", identifiers)
        self.assertIn("C0-IMMUTABLE-LEGACY-BASELINE", identifiers)

    def test_qualification_failures_use_specific_contract_ids(self):
        cases = (
            ("use crate::r#Value as V;", "C0-RAW-AUDITED-ALIAS"),
            ("use crate::Ref as R;", "C0-REF-ALIAS"),
            ("use crate::kind::Kind as K;", "C0-SEMANTIC-KIND-ALIAS"),
            ("use crate::*; fn f() { Kind::Any; }", "C0-KIND-QUALIFIER-AMBIGUOUS"),
        )
        for source, expected in cases:
            with self.subTest(expected=expected):
                path = self.write("src/core/src/new.rs", source)
                failures = CHECKER.qualification_failures(self.root, self.inventory)
                self.assertIn(expected, self.ids(failures))
                path.unlink()

    def test_frozen_ref_alias_definition_cannot_change(self):
        baseline = {
            "legacy_aliases": [
                {
                    "name": "ValRef",
                    "target": "Ref<Value>",
                    "path": "src/core/src/types/mod.rs",
                    "line": 121,
                }
            ]
        }
        live = copy.deepcopy(baseline)
        live["legacy_aliases"][0]["target"] = "Ref<Other>"
        self.assertIn(
            "C0-IMMUTABLE-LEGACY-BASELINE",
            self.ids(
                CHECKER.legacy_alias_baseline_failures(
                    baseline, live, Path("baseline.json")
                )
            ),
        )

    def test_legacy_scanner_contract_cannot_drift_with_baseline(self):
        baseline = copy.deepcopy(self.baseline)
        baseline["scanner_contract"]["implementation_sha256"] = "0" * 64
        self.assertIn(
            "C0-LEGACY-SCANNER-DRIFT",
            self.ids(
                CHECKER.immutable_baseline_failures(
                    self.root,
                    baseline,
                    self.baseline_path,
                    verify_git=False,
                )
            ),
        )

    def test_runtime_representation_definition_cannot_become_kind_scheme(self):
        path = self.root / "src/core/src/function/signature.rs"
        source = path.read_text(encoding="utf-8").replace(
            "pub trait FunctionRuntimeType {}",
            "pub type FunctionRuntimeType = KindScheme;",
        )
        path.write_text(source, encoding="utf-8")
        self.assertIn("C0-KIND-SCHEME-SEPARATION", self.ids(self.audit()))

    def test_new_kind_variant_without_target_fails(self):
        source = self.root.joinpath("src/core/src/kind.rs").read_text(encoding="utf-8")
        self.write("src/core/src/kind.rs", source.replace("}\n", "    Added,\n}\n"))
        self.assertIn("C0-KIND-COVERAGE", self.ids(self.audit()))

    def test_new_kind_use_without_classification_fails(self):
        self.write(
            "src/core/src/new.rs",
            "use crate::kind::Kind; fn new() { let _ = Kind::Any; }\n",
        )
        self.assertIn("C0-OCCURRENCE-CLASSIFICATION", self.ids(self.audit()))

    def test_value_kind_maps_to_reified_snapshot(self):
        self.assertEqual(CHECKER.frozen_semantics_failures(self.migration, self.migration_path), [])
        item = next(t for f in self.migration["families"] for t in f["targets"] if t["id"] == "reified-type-snapshot")
        item["semantic_category"] = "schema"
        self.assertIn("C0-FROZEN-SEMANTICS", self.ids(CHECKER.frozen_semantics_failures(self.migration, self.migration_path)))

    def test_kind_reference_maps_to_binding_contract(self):
        row = next(r for r in self.migration["use_classifications"] if r["enum"] == "Kind" and r["variant"] == "Reference")
        row["target"] = "mutable-reference-runtime-storage"
        self.assertIn("C0-FROZEN-SEMANTICS", self.ids(CHECKER.frozen_semantics_failures(self.migration, self.migration_path)))

    def test_value_kind_any_does_not_produce_schema(self):
        item = next(t for f in self.migration["families"] for t in f["targets"] if t["id"] == "kind-wildcard")
        item["semantic_category"] = "schema"
        self.assertIn("C0-FROZEN-SEMANTICS", self.ids(CHECKER.frozen_semantics_failures(self.migration, self.migration_path)))

    def test_new_occurrence_without_classification_fails(self):
        self.write("src/core/src/new.rs", "fn new() { let _ = Value::Empty; }\n")
        self.assertIn("C0-OCCURRENCE-CLASSIFICATION", self.ids(self.audit()))

    def test_duplicate_classification_fails(self):
        self.migration["use_classifications"].append(copy.deepcopy(self.migration["use_classifications"][0]))
        self.save_path(self.migration_path, self.migration)
        self.assertIn("C0-OCCURRENCE-CLASSIFICATION", self.ids(self.audit()))

    def test_stale_classification_fails(self):
        self.migration["use_classifications"][0]["sites"][0]["line"] = 999
        self.save_path(self.migration_path, self.migration)
        self.assertIn("C0-OCCURRENCE-CLASSIFICATION", self.ids(self.audit()))

    def test_same_file_two_variants_can_have_different_roles(self):
        empty = next(r for r in self.migration["use_classifications"] if r["enum"] == "LegacyValue" and r["variant"] == "Empty")
        matrix = next(r for r in self.migration["use_classifications"] if r["enum"] == "LegacyValue" and r["variant"] == "MatrixValue")
        empty["roles"] = ["machine-output"]
        matrix["roles"] = ["compiler-type-data"]
        self.save_path(self.migration_path, self.migration)
        self.assertEqual(self.audit(), [])

    def test_same_variant_in_one_file_can_have_separate_targets(self):
        targets = {
            row["target"]
            for row in self.migration["use_classifications"]
            if row["enum"] == "LegacyValue" and row["variant"] == "Empty"
        }
        self.assertEqual(len(targets), 6)

    def test_ambiguous_target_terms_fail(self):
        for phrase in ("thing-or-other", "either result", "one-of-results"):
            with self.subTest(phrase=phrase):
                migration = copy.deepcopy(self.migration)
                migration["families"][0]["targets"][0]["representation"] = phrase
                self.assertIn("C0-AMBIGUOUS-TARGET", self.ids(CHECKER.family_contract_failures(self.inventory, migration, self.migration_path)))

    def test_partitioned_family_missing_target_fails(self):
        self.migration["families"][1]["targets"] = [
            item for item in self.migration["families"][1]["targets"] if item["id"] != "option-absence"
        ]
        failures = CHECKER.occurrence_classification_failures(self.inventory, self.migration, self.migration_path)
        self.assertIn("C0-TARGET-MEMBERSHIP", self.ids(failures))

    def test_frozen_target_contracts_cannot_be_rewritten(self):
        mutations = (
            ("option-absence", "semantic_category", "execution-control"),
            ("homogeneous-matrix-snapshot", "implementation_gate", "C3"),
            ("matrix-construction-ir", "representation", "OtherIR"),
        )
        for target_id, field, replacement in mutations:
            with self.subTest(target=target_id, field=field):
                migration = copy.deepcopy(self.migration)
                item = next(
                    target
                    for family in migration["families"]
                    for target in family["targets"]
                    if target["id"] == target_id
                )
                item[field] = replacement
                self.assertIn(
                    "C0-FROZEN-TARGET-DRIFT",
                    self.ids(
                        CHECKER.frozen_target_failures(
                            migration,
                            self.frozen_targets,
                            self.migration_path,
                            self.frozen_targets_path,
                        )
                    ),
                )

    def test_regenerated_current_inventory_does_not_approve_legacy_growth(self):
        self.write("src/core/src/new.rs", "fn added(_: ValRef) {}\n")
        regenerated = CHECKER.GENERATOR.generate(self.root, self.reference)
        self.save_path(self.inventory_path, regenerated)
        self.assertIn("C0-LEGACY-GROWTH", self.ids(self.audit()))


class LegacyGrowthTests(unittest.TestCase):
    def row(self, path, count):
        return {
            "path": path,
            "count": count,
            "sites": [
                {
                    "line": 1,
                    "column": index + 1,
                    "fingerprint": f"{index + 1:064x}",
                }
                for index in range(count)
            ],
        }

    def failures(self, baseline_rows, live_rows, identifier="valref-alias"):
        baseline = {"high_risk_api_uses": {identifier: baseline_rows}}
        live = {"high_risk_api_uses": {identifier: live_rows}}
        return CHECKER.high_risk_failures(baseline, live, Path("baseline.json"))

    def test_new_legacy_use_fails(self):
        self.assertTrue(self.failures([self.row("a.rs", 1)], [self.row("a.rs", 2)]))

    def test_deleted_legacy_use_passes(self):
        self.assertEqual(self.failures([self.row("a.rs", 1)], []), [])

    def test_count_decrease_passes(self):
        self.assertEqual(self.failures([self.row("a.rs", 2)], [self.row("a.rs", 1)]), [])

    def test_new_path_fails(self):
        self.assertTrue(self.failures([self.row("a.rs", 1)], [self.row("b.rs", 1)]))

    def test_same_file_substitution_fails(self):
        baseline = {
            "path": "a.rs",
            "count": 1,
            "sites": [{"line": 1, "column": 1, "fingerprint": "a" * 64}],
        }
        live = {
            "path": "a.rs",
            "count": 1,
            "sites": [{"line": 2, "column": 1, "fingerprint": "b" * 64}],
        }
        self.assertTrue(self.failures([baseline], [live]))

    def test_new_reactive_cell_id_path_fails(self):
        self.assertTrue(
            self.failures([], [self.row("new.rs", 1)], "reactive-cell-id")
        )

    def test_new_value_state_journal_path_fails(self):
        self.assertTrue(
            self.failures([], [self.row("new.rs", 1)], "value-state-journal")
        )

    def test_exact_post_c0_authorization_passes_but_excess_authorization_fails(self):
        live = self.row("new.rs", 1)
        authorization = {
            "gate": "D1A",
            "identifier": "valref-alias",
            "path": "new.rs",
            "fingerprint": live["sites"][0]["fingerprint"],
            "count": 1,
            "reason": "authorized compiler adapter boundary",
        }
        baseline = {"high_risk_api_uses": {"valref-alias": []}}
        current = {"high_risk_api_uses": {"valref-alias": [live]}}
        migration = {"authorized_high_risk_uses": [authorization]}
        self.assertEqual(
            CHECKER.high_risk_failures(
                baseline,
                current,
                Path("baseline.json"),
                migration,
                Path("migration.json"),
            ),
            [],
        )

        authorization["count"] = 2
        self.assertIn(
            "C0-LEGACY-AUTHORIZATION-STALE",
            {
                item.contract_id
                for item in CHECKER.high_risk_failures(
                    baseline,
                    current,
                    Path("baseline.json"),
                    migration,
                    Path("migration.json"),
                )
            },
        )


class GateBFreshnessTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.evidence_path = self.root / "benchmarks/runtime/gate-b/b2-resident-turn.json"
        self.evidence_path.parent.mkdir(parents=True)
        self.evidence_path.write_bytes((ROOT / "benchmarks/runtime/gate-b/b2-resident-turn.json").read_bytes())
        protected = self.root / "src/runtime/src/ledger/state.rs"
        protected.parent.mkdir(parents=True)
        protected.write_text("base\n", encoding="utf-8")
        unprotected = self.root / "docs/notes.md"
        unprotected.parent.mkdir(parents=True)
        unprotected.write_text("base\n", encoding="utf-8")
        checker = self.root / "scripts/check-value-system-contract.py"
        checker.parent.mkdir(parents=True)
        checker.write_text("base\n", encoding="utf-8")
        self.gate_contract = (
            self.root / "tests/architecture/value-system/gate-b-regression.json"
        )
        self.gate_contract.parent.mkdir(parents=True)
        self.gate_contract.write_text("{}\n", encoding="utf-8")
        frozen_vectors = (
            self.root
            / "tests/architecture/value-system/canonical-encoding-v1-vectors.json"
        )
        frozen_vectors.write_text("base\n", encoding="utf-8")
        self.run_command("git", "init", "-q")
        self.run_command("git", "config", "user.email", "fixture@example.com")
        self.run_command("git", "config", "user.name", "Fixture")
        self.commit("base")
        self.base = self.output("git", "rev-parse", "HEAD")
        self.contract = CHECKER.load_json(CONTRACTS / "gate-b-regression.json")
        self.refresh_evidence(self.base)
        self.commit("evidence")

    def run_command(self, *args):
        subprocess.run(args, cwd=self.root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def output(self, *args):
        return subprocess.run(args, cwd=self.root, check=True, stdout=subprocess.PIPE, text=True).stdout.strip()

    def commit(self, message):
        self.run_command("git", "add", ".")
        self.run_command("git", "commit", "-q", "-m", message)

    def refresh_evidence(self, commit):
        report = json.loads(self.evidence_path.read_text(encoding="utf-8"))
        report["git_commit"] = commit
        self.evidence_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        self.contract["evidence_commit"] = commit
        self.contract["evidence_sha256"] = hashlib.sha256(self.evidence_path.read_bytes()).hexdigest()

    def failures(self):
        return CHECKER.gate_b_failures(self.root, self.contract, Path("gate-b-regression.json"), enforce_freshness=True)

    def test_ledger_change_fails(self):
        self.write_protected("src/runtime/src/ledger/state.rs")
        self.assertIn("C0-GATE-B-EVIDENCE-STALE", {item.contract_id for item in self.failures()})

    def test_benchmark_runner_change_fails(self):
        self.write_protected("scripts/run-gate-b-benchmarks.py")
        self.assertIn("C0-GATE-B-EVIDENCE-STALE", {item.contract_id for item in self.failures()})

    def test_cargo_lock_change_fails(self):
        self.write_protected("Cargo.lock")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_engine_module_routing_change_fails(self):
        self.write_protected("src/engine/src/lib.rs")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_any_semantic_core_change_fails(self):
        self.write_protected("src/core/src/snapshot.rs")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_frozen_vector_change_fails_gate_b_freshness(self):
        self.write_protected(
            "tests/architecture/value-system/canonical-encoding-v1-vectors.json"
        )
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_required_routing_path_cannot_be_removed_from_contract(self):
        self.contract["protected_paths"]["exact"].remove("src/engine/src/lib.rs")
        self.assertIn(
            "C0-GATE-B-PROTECTION-DRIFT",
            {item.contract_id for item in self.failures()},
        )

    def test_gate_b_checker_change_fails_even_if_contract_omits_it(self):
        self.write_protected("scripts/check-value-system-contract.py")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_gate_b_contract_change_fails_even_if_it_omits_itself(self):
        self.write_protected(
            "tests/architecture/value-system/gate-b-regression.json"
        )
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_fresh_evidence_pointer_only_contract_change_passes(self):
        self.gate_contract.write_text(
            json.dumps(self.contract, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        self.commit("contract baseline")
        self.write_protected("src/runtime/src/ledger/state.rs")
        protected_commit = self.output("git", "rev-parse", "HEAD")
        self.refresh_evidence(protected_commit)
        self.gate_contract.write_text(
            json.dumps(self.contract, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        self.commit("fresh evidence pointers")
        self.assertEqual(self.failures(), [])

    def test_renaming_protected_file_outside_scope_fails(self):
        destination = self.root / "docs/moved-ledger-state.rs"
        destination.parent.mkdir(parents=True, exist_ok=True)
        self.run_command(
            "git",
            "mv",
            "src/runtime/src/ledger/state.rs",
            "docs/moved-ledger-state.rs",
        )
        self.commit("rename protected file")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_deleting_protected_file_fails(self):
        self.root.joinpath("src/runtime/src/ledger/state.rs").unlink()
        self.commit("delete protected file")
        self.assertIn(
            "C0-GATE-B-EVIDENCE-STALE",
            {item.contract_id for item in self.failures()},
        )

    def test_renaming_unprotected_file_does_not_require_fresh_evidence(self):
        self.run_command("git", "mv", "docs/notes.md", "docs/renamed-notes.md")
        self.commit("rename unprotected file")
        self.assertEqual(self.failures(), [])

    def write_protected(self, relative):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("changed\n", encoding="utf-8")
        self.commit("protected change")

    def test_fresh_evidence_containing_protected_change_passes(self):
        self.write_protected("src/runtime/src/ledger/state.rs")
        protected_commit = self.output("git", "rev-parse", "HEAD")
        self.refresh_evidence(protected_commit)
        self.commit("fresh evidence")
        self.assertEqual(self.failures(), [])

    def test_fresh_evidence_after_protected_rename_passes(self):
        destination = self.root / "docs/moved-ledger-state.rs"
        destination.parent.mkdir(parents=True, exist_ok=True)
        self.run_command(
            "git",
            "mv",
            "src/runtime/src/ledger/state.rs",
            "docs/moved-ledger-state.rs",
        )
        self.commit("rename protected file")
        renamed_commit = self.output("git", "rev-parse", "HEAD")
        self.refresh_evidence(renamed_commit)
        self.commit("fresh evidence after rename")
        self.assertEqual(self.failures(), [])


class C2AdapterAllowanceTests(unittest.TestCase):
    def setUp(self):
        self.approved = CHECKER.load_json(
            CONTRACTS / "c2-legacy-adapter-boundary.json"
        )
        self.approved_path = CONTRACTS / "c2-legacy-adapter-boundary.json"

    def root_with_boundary(self, source):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        path = root / "src/core/src/legacy_adapter/value.rs"
        path.parent.mkdir(parents=True)
        path.write_text(source, encoding="utf-8")
        return root

    def ids(self, root):
        return {
            item.contract_id
            for item in CHECKER.c2_adapter_boundary_failures(
                root, self.approved, self.approved_path
            )
        }

    def test_checked_in_adapter_allowance_matches(self):
        self.assertEqual(self.ids(ROOT), set())

    def test_approved_adapter_uses_may_all_disappear(self):
        self.assertEqual(self.ids(self.root_with_boundary("")), set())

    def test_new_adapter_use_requires_an_explicit_allowance(self):
        source = (ROOT / "src/core/src/legacy_adapter/value.rs").read_text(
            encoding="utf-8"
        )
        root = self.root_with_boundary(
            source + "\nfn unapproved(value: &LegacyValue) { let _ = value; }\n"
        )
        self.assertIn("C2-ADAPTER-ALLOWANCE", self.ids(root))


class BoundaryAndReportingTests(unittest.TestCase):
    def root_with(self, relative, source):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        path = root / relative
        path.parent.mkdir(parents=True)
        path.write_text(source, encoding="utf-8")
        return root

    def ids(self, root):
        return {item.contract_id for item in CHECKER.future_boundary_failures(root)}

    def test_snapshot_cannot_import_legacy_identity(self):
        self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(self.root_with("src/core/src/snapshot/new.rs", "use crate::Ref;\n")))

    def test_file_form_snapshot_cannot_import_legacy_identity(self):
        self.assertIn(
            "C0-SNAPSHOT-LEGACY-IMPORT",
            self.ids(self.root_with("src/core/src/snapshot.rs", "use crate::Ref;\n")),
        )

    def test_snapshot_cannot_use_compatibility_ref_alias(self):
        root = self.root_with(
            "src/core/src/snapshot/new.rs",
            "use crate::SymbolTableRef;\nstruct Snapshot(SymbolTableRef);\n",
        )
        self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(root))

    def test_snapshot_cannot_rename_compatibility_ref_alias(self):
        root = self.root_with(
            "src/core/src/snapshot/new.rs",
            "use crate::SymbolTableRef as Hidden;\nstruct Snapshot(Hidden);\n",
        )
        self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(root))

    def test_snapshot_cannot_hide_compatibility_ref_behind_local_alias(self):
        root = self.root_with(
            "src/core/src/snapshot/new.rs",
            "type Hidden = SymbolTableRef;\nstruct Snapshot(Hidden);\n",
        )
        self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(root))

    def test_snapshot_rejects_every_c2_mutable_or_physical_dependency(self):
        for type_name in (
            "LegacyValue",
            "ValueKind",
            "MutableReference",
            "ReactiveCellId",
            "StateArena",
            "RuntimeExecutionTransaction",
            "ValueStateJournal",
            "ReactiveTurnJournal",
            "DMatrix",
            "Rc",
            "RefCell",
            "Cell",
            "UnsafeCell",
            "Mutex",
            "RwLock",
            "AtomicU64",
        ):
            with self.subTest(type_name=type_name):
                root = self.root_with(
                    "src/core/src/snapshot/forbidden.rs",
                    f"pub struct Bad({type_name});\n",
                )
                self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(root))

    def test_snapshot_rejects_transitive_wrappers_and_renamed_imports(self):
        root = self.root_with(
            "src/core/src/mutable_wrapper.rs",
            "pub struct Hidden(StateArena);\npub type Reexport = Hidden;\n",
        )
        snapshot = root / "src/core/src/snapshot/hidden.rs"
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_text(
            "use crate::mutable_wrapper::Reexport as ApparentlyImmutable;\n"
            "pub struct Bad(ApparentlyImmutable);\n",
            encoding="utf-8",
        )
        self.assertIn("C0-SNAPSHOT-LEGACY-IMPORT", self.ids(root))

    def test_general_snapshot_values_cannot_derive_relations(self):
        for type_name, declaration in (
            ("Value", "struct"),
            ("ValueData", "enum"),
        ):
            with self.subTest(type_name=type_name):
                source = f"#[derive(Clone, PartialEq, Eq, Hash, Ord)]\npub {declaration} {type_name} {{}}\n"
                self.assertIn(
                    "C2-EXPLICIT-VALUE-RELATIONS",
                    self.ids(self.root_with("src/core/src/snapshot/value.rs", source)),
                )

    def test_general_snapshot_values_cannot_gain_manual_or_aliased_relations(self):
        for source in (
            "impl PartialEq for Value { fn eq(&self, _: &Self) -> bool { true } }\n",
            "type Hidden = ValueData;\nimpl core::hash::Hash for Hidden { fn hash<H>(&self, _: &mut H) {} }\n",
            "use core::cmp::Ord as Compare;\nimpl Compare for Value { fn cmp(&self, _: &Self) -> core::cmp::Ordering { todo!() } }\n",
        ):
            with self.subTest(source=source):
                self.assertIn(
                    "C2-EXPLICIT-VALUE-RELATIONS",
                    self.ids(self.root_with("src/core/src/snapshot/value.rs", source)),
                )

    def test_schema_cannot_depend_on_runtime(self):
        self.assertIn("C0-SCHEMA-DEPENDENCY", self.ids(self.root_with("src/core/src/schema/new.rs", "use mech_runtime::Runtime;\n")))

    def test_file_form_schema_cannot_depend_on_runtime(self):
        self.assertIn(
            "C0-SCHEMA-DEPENDENCY",
            self.ids(
                self.root_with(
                    "src/core/src/schema.rs", "use mech_runtime::Runtime;\n"
                )
            ),
        )

    def test_c1_semantic_modules_reject_mutable_value_and_layout_dependencies(self):
        for relative, source in (
            ("src/core/src/semantic_identity.rs", "struct Bad(ValueKind);\n"),
            ("src/core/src/nominal.rs", "use crate::Ref;\n"),
            ("src/core/src/dimension.rs", "struct Bad(ReactiveCellId);\n"),
            ("src/core/src/kind_expr.rs", "use nalgebra::DMatrix;\n"),
            ("src/core/src/kind_scheme.rs", "struct Bad(StateArena);\n"),
            ("src/core/src/schema/new.rs", "struct Bad(ValueStateJournal);\n"),
            ("src/core/src/schema/new.rs", "fn stride() {}\n"),
        ):
            with self.subTest(relative=relative, source=source):
                self.assertIn(
                    "C1-SEMANTIC-BOUNDARY",
                    self.ids(self.root_with(relative, source)),
                )

    def test_c1_semantic_modules_resolve_transitive_aliases_and_wrappers(self):
        root = self.root_with(
            "src/core/src/storage_alias.rs",
            "pub struct Hidden(StateArena);\npub type Reexport = Hidden;\n",
        )
        semantic = root / "src/core/src/schema/hidden.rs"
        semantic.parent.mkdir(parents=True, exist_ok=True)
        semantic.write_text(
            "use crate::storage_alias::Reexport as Innocent;\n"
            "pub struct Bad(Innocent);\n",
            encoding="utf-8",
        )
        self.assertIn("C1-SEMANTIC-BOUNDARY", self.ids(root))

    def test_c1_semantic_modules_reject_standard_interior_mutability(self):
        for type_name in (
            "UnsafeCell",
            "Cell",
            "RefCell",
            "Mutex",
            "RwLock",
            "Once",
            "OnceCell",
            "OnceLock",
            "LazyCell",
            "LazyLock",
            "Barrier",
            "Condvar",
            "AtomicU64",
            "AtomicPtr",
        ):
            with self.subTest(type_name=type_name):
                field_type = (
                    type_name
                    if type_name in {"Once", "Barrier", "Condvar", "AtomicU64"}
                    else f"{type_name}<u8>"
                )
                source = f"pub struct Bad({field_type});\n"
                self.assertIn(
                    "C1-SEMANTIC-BOUNDARY",
                    self.ids(self.root_with("src/core/src/schema/interior.rs", source)),
                )

    def test_c1_semantic_modules_reject_renamed_and_wrapped_interior_mutability(self):
        root = self.root_with(
            "src/core/src/interior_wrapper.rs",
            "use std::sync::Mutex as Lock;\npub struct Hidden(Lock<u8>);\n",
        )
        semantic = root / "src/core/src/schema/interior.rs"
        semantic.parent.mkdir(parents=True, exist_ok=True)
        semantic.write_text(
            "use crate::interior_wrapper::Hidden as ApparentlyImmutable;\n"
            "pub struct Bad(ApparentlyImmutable);\n",
            encoding="utf-8",
        )
        self.assertIn("C1-SEMANTIC-BOUNDARY", self.ids(root))

    def test_finalized_semantic_types_cannot_regain_derived_deserialize(self):
        for relative, type_name in (
            ("src/core/src/nominal.rs", "CanonicalNominalPath"),
            ("src/core/src/dimension.rs", "DimensionParameter"),
            ("src/core/src/kind_scheme.rs", "KindScheme"),
            ("src/core/src/schema/mod.rs", "Schema"),
            ("src/core/src/schema/shape.rs", "ShapeInstance"),
        ):
            with self.subTest(type_name=type_name):
                source = (
                    '#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]\n'
                    '#[derive(Clone)]\n'
                    f"pub struct {type_name};\n"
                )
                self.assertIn(
                    "C1-FINALIZED-SERDE-BOUNDARY",
                    self.ids(self.root_with(relative, source)),
                )

    def test_finalized_semantic_types_cannot_gain_manual_deserialize(self):
        for relative, type_name in (
            ("src/core/src/nominal.rs", "CanonicalNominalPath"),
            ("src/core/src/dimension.rs", "DimensionParameter"),
            ("src/core/src/kind_scheme.rs", "KindScheme"),
            ("src/core/src/schema/mod.rs", "Schema"),
            ("src/core/src/schema/shape.rs", "ShapeInstance"),
            ("src/core/src/schema/table.rs", "SchemaHandle"),
        ):
            with self.subTest(relative=relative, type_name=type_name):
                source = f"impl<'de> Deserialize<'de> for {type_name} {{}}\n"
                self.assertIn(
                    "C1-FINALIZED-SERDE-BOUNDARY",
                    self.ids(self.root_with(relative, source)),
                )

    def test_manual_deserialize_check_resolves_qualified_and_aliased_names(self):
        cases = (
            (
                "src/core/src/schema/mod.rs",
                "impl<'de> serde::de::Deserialize<'de> for crate::Schema {}\n",
            ),
            (
                "src/core/src/schema/shape.rs",
                "use serde::de::Deserialize as Decode;\n"
                "impl<'de> Decode<'de> for ShapeInstance {}\n",
            ),
            (
                "src/core/src/kind_scheme.rs",
                "type Hidden = KindScheme;\n"
                "impl<'de> Deserialize<'de> for Hidden {}\n",
            ),
        )
        for relative, source in cases:
            with self.subTest(relative=relative, source=source):
                self.assertIn(
                    "C1-FINALIZED-SERDE-BOUNDARY",
                    self.ids(self.root_with(relative, source)),
                )

    def test_non_final_semantic_types_may_implement_deserialize(self):
        source = "impl<'de> Deserialize<'de> for SchemaDraft {}\n"
        self.assertNotIn(
            "C1-FINALIZED-SERDE-BOUNDARY",
            self.ids(self.root_with("src/core/src/schema/mod.rs", source)),
        )

    def test_finalized_snapshot_types_cannot_regain_deserialize(self):
        for relative, type_name, declaration in (
            ("src/core/src/snapshot/validation.rs", "Value", "struct"),
            ("src/core/src/snapshot/data.rs", "ValueData", "enum"),
            ("src/core/src/snapshot/data.rs", "MapValue", "struct"),
            ("src/core/src/snapshot/constants.rs", "ConstantHandle", "struct"),
        ):
            with self.subTest(type_name=type_name):
                source = (
                    '#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]\n'
                    f"pub {declaration} {type_name} {{}}\n"
                )
                self.assertIn(
                    "C2-FINALIZED-SNAPSHOT-SERDE",
                    self.ids(self.root_with(relative, source)),
                )

    def test_open_snapshot_drafts_and_bit_wrappers_keep_serde(self):
        for relative, type_name, declaration in (
            ("src/core/src/snapshot/draft.rs", "ValueDraft", "struct"),
            ("src/core/src/snapshot/draft.rs", "ValueDataDraft", "enum"),
            ("src/core/src/snapshot/data.rs", "F32Bits", "struct"),
        ):
            with self.subTest(type_name=type_name):
                source = (
                    '#[cfg_attr(feature = "serde", derive(Serialize))]\n'
                    f"pub {declaration} {type_name} {{}}\n"
                )
                self.assertIn(
                    "C1-OPEN-SERDE-BOUNDARY",
                    self.ids(self.root_with(relative, source)),
                )

    def test_resident_paths_cannot_acquire_snapshot_work(self):
        root = self.root_with(
            "src/engine/src/resident/turn.rs",
            "fn turn(store: ConstantStore, value: ValueDraft) {}\n",
        )
        self.assertIn("C2-RESIDENT-LEGACY-HOT-PATH", self.ids(root))

    def test_general_resident_kernel_cannot_import_snapshot_helpers(self):
        root = self.root_with(
            "src/engine/src/resident/turn.rs",
            "use mech_core::snapshot::build_f64_set_snapshot;\n",
        )
        self.assertIn("C2-RESIDENT-LEGACY-HOT-PATH", self.ids(root))

    def test_finalized_snapshot_kernel_imports_are_exactly_allowlisted(self):
        root = self.root_with(
            "src/engine/src/resident/set.rs",
            "use mech_core::snapshot::{build_f64_set_snapshot, "
            "build_f64_set_snapshot_after_remove, f64_set_snapshot_contains};\n",
        )
        self.assertNotIn("C2-RESIDENT-LEGACY-HOT-PATH", self.ids(root))

        root = self.root_with(
            "src/engine/src/resident/composite.rs",
            "use mech_core::snapshot::{F64Bits, MatrixValue, rebuild_composite_snapshot};\n",
        )
        self.assertNotIn("C2-RESIDENT-LEGACY-HOT-PATH", self.ids(root))

    def test_finalized_snapshot_kernel_cannot_import_draft_or_hash_work(self):
        root = self.root_with(
            "src/engine/src/resident/set.rs",
            "use mech_core::snapshot::{build_f64_set_snapshot, ValueDraft};\n",
        )
        self.assertIn("C2-RESIDENT-LEGACY-HOT-PATH", self.ids(root))

    def test_open_semantic_syntax_must_keep_deserialize(self):
        source = (
            '#[cfg_attr(feature = "serde", derive(Serialize))]\n'
            '#[derive(Clone)]\n'
            "pub enum KindExpr { Id }\n"
        )
        self.assertIn(
            "C1-OPEN-SERDE-BOUNDARY",
            self.ids(self.root_with("src/core/src/kind_expr.rs", source)),
        )

    def test_schema_handles_cannot_gain_standalone_serde_construction(self):
        source = (
            '#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]\n'
            '#[derive(Clone)]\n'
            "pub struct SchemaHandle(u32);\n"
        )
        self.assertIn(
            "C1-EPHEMERAL-HANDLE-SERDE-BOUNDARY",
            self.ids(self.root_with("src/core/src/schema/table.rs", source)),
        )

    def test_semantic_builders_cannot_panic_on_identity_or_input(self):
        for relative, source in (
            (
                "src/core/src/dimension.rs",
                "pub fn declare() -> DimensionParameterId { assert!(false); todo!() }\n"
                "pub fn declarations() {}\n",
            ),
            (
                "src/core/src/schema/table.rs",
                "pub fn insert() -> SchemaHandle { value.expect(\"id\") }\n"
                "pub fn finish() {}\n",
            ),
        ):
            with self.subTest(relative=relative):
                self.assertIn(
                    "C1-SEMANTIC-BUILDER-ERROR",
                    self.ids(self.root_with(relative, source)),
                )

    def test_canonical_vectors_must_use_validated_finalization_and_shape_apis(self):
        root = self.root_with(
            "src/core/tests/canonical_schema_vectors.rs",
            "fn vectors() { draft.finalize(); }\n",
        )
        self.assertIn("C1-FINALIZED-CONSTRUCTION", self.ids(root))

    def test_c1_validation_routes_cannot_be_removed(self):
        for relative, source in (
            ("src/core/src/kind_expr.rs", "fn canonical_closed_kind_bytes() {}\n"),
            ("src/core/src/kind_scheme.rs", "fn new() {}\n"),
            ("src/core/src/legacy_adapter/kind.rs", "fn kind_expr_from_legacy() {}\n"),
            ("src/core/src/schema/shape.rs", "fn evaluate_body_extents() {}\n"),
        ):
            with self.subTest(relative=relative):
                self.assertIn(
                    "C1-VALIDATED-CONSTRUCTION-ROUTE",
                    self.ids(self.root_with(relative, source)),
                )

    def test_legacy_adapter_is_the_explicit_kind_value_kind_exception(self):
        root = self.root_with(
            "src/core/src/legacy_adapter/kind.rs",
            "fn adapt(kind: Kind, value: ValueKind) {}\n",
        )
        self.assertNotIn("C1-SEMANTIC-BOUNDARY", self.ids(root))

    def test_legacy_adapter_cannot_hide_future_variants_behind_catch_all(self):
        for fallback in (
            "_ => ()",
            "other => drop(other)",
            "other @ _ => drop(other)",
            "_ | Kind::Any => ()",
            "Kind::Any | _ => ()",
            "| Kind::Any | _ => ()",
            "(other) | Kind::Any => drop(other)",
            "other @ (_ | Kind::Any) => drop(other)",
            "#[allow(unreachable_patterns)] (_ | Kind::Any) => ()",
            "#[allow(unused_variables)] other => drop(other)",
            "#[cfg_attr(feature = \"strict\", allow(unused_variables))] mut other => drop(other)",
            "#[cfg(any())] #[allow(unused_variables)] ref other @ _ => drop(other)",
        ):
            with self.subTest(fallback=fallback):
                root = self.root_with(
                    "src/core/src/legacy_adapter/kind.rs",
                    f"fn adapt(kind: Kind) {{ match kind {{ Kind::Any => (), {fallback} }} }}\n",
                )
                self.assertIn("C1-LEGACY-ADAPTER-EXHAUSTIVE", self.ids(root))

    def test_legacy_adapter_allows_explicit_or_patterns_and_guarded_wildcards(self):
        for fallback in (
            "Kind::Any | Kind::Empty => ()",
            "_ if condition => ()",
        ):
            with self.subTest(fallback=fallback):
                root = self.root_with(
                    "src/core/src/legacy_adapter/kind.rs",
                    f"fn adapt(kind: Kind) {{ match kind {{ {fallback} }} }}\n",
                )
                self.assertNotIn("C1-LEGACY-ADAPTER-EXHAUSTIVE", self.ids(root))

    def test_exact_c2_value_adapter_may_mention_both_representations(self):
        root = self.root_with("src/core/src/legacy_adapter/value.rs", "fn convert(_: LegacyValue) -> snapshot::Value { todo!() }\n")
        self.assertNotIn("C0-ADAPTER-COEXISTENCE", self.ids(root))

    def test_other_legacy_adapter_cannot_mention_both_representations(self):
        root = self.root_with("src/core/src/legacy_adapter/convert.rs", "fn convert(_: LegacyValue) -> snapshot::Value { todo!() }\n")
        self.assertIn("C0-ADAPTER-COEXISTENCE", self.ids(root))

    def test_non_adapter_cannot_mention_both_representations(self):
        root = self.root_with("src/core/src/convert.rs", "fn convert(_: LegacyValue) -> snapshot::Value { todo!() }\n")
        self.assertIn("C0-ADAPTER-COEXISTENCE", self.ids(root))

    def test_blanket_conversion_fails(self):
        root = self.root_with("src/core/src/legacy_adapter/blanket.rs", "impl From<LegacyValue> for snapshot::Value { fn from(_: LegacyValue) -> Self { todo!() } }\n")
        self.assertIn("C0-BLANKET-CONVERSION", self.ids(root))

    def test_qualified_blanket_conversion_fails(self):
        root = self.root_with("src/core/src/legacy_adapter/blanket.rs", "impl core::convert::From<LegacyValue> for snapshot::Value { fn from(_: LegacyValue) -> Self { todo!() } }\n")
        self.assertIn("C0-BLANKET-CONVERSION", self.ids(root))

    def test_aliased_blanket_conversion_fails(self):
        root = self.root_with(
            "src/core/src/legacy_adapter/blanket.rs",
            (
                "type Snap = snapshot::Value;\n"
                "type Legacy = LegacyValue;\n"
                "impl From<Legacy> for Snap { fn from(_: Legacy) -> Self { todo!() } }\n"
            ),
        )
        self.assertIn("C0-BLANKET-CONVERSION", self.ids(root))

    def test_import_renamed_blanket_conversion_fails(self):
        root = self.root_with(
            "src/core/src/legacy_adapter/blanket.rs",
            (
                "use crate::legacy_value::LegacyValue as Legacy;\n"
                "use crate::snapshot::Value as Snap;\n"
                "impl Into<Snap> for Legacy { fn into(self) -> Snap { todo!() } }\n"
            ),
        )
        self.assertIn("C0-BLANKET-CONVERSION", self.ids(root))

    def test_import_renamed_conversion_trait_fails(self):
        root = self.root_with(
            "src/core/src/legacy_adapter/blanket.rs",
            (
                "use std::convert::From as ConvertFrom;\n"
                "impl ConvertFrom<LegacyValue> for snapshot::Value { "
                "fn from(_: LegacyValue) -> Self { todo!() } }\n"
            ),
        )
        self.assertIn("C0-BLANKET-CONVERSION", self.ids(root))

    def test_engine_artifact_cannot_accept_legacy_value(self):
        root = self.root_with("src/engine/src/resident/artifact.rs", "fn artifact(_: LegacyValue) {}\n")
        self.assertIn("C0-ENGINE-LEGACY-ARTIFACT", self.ids(root))

    def test_compiler_artifact_adapter_may_accept_legacy_value(self):
        root = self.root_with(
            "src/engine/src/artifact/compiler.rs",
            "fn compile_executable_program_artifact(_: LegacyValue) {}\n",
        )
        self.assertNotIn("C0-ENGINE-LEGACY-ARTIFACT", self.ids(root))

    def test_failure_render_includes_every_required_field(self):
        item = CHECKER.failure(
            "C0-FIXTURE", "subject", "src/core/src/value.rs", "classified", "missing",
            "tests/architecture/value-system/migration.json", 12, 7, "LegacyValue", "Added"
        )
        rendered = item.render()
        for field in (
            "C0-FIXTURE", "enum=LegacyValue", "variant=Added", "path=src/core/src/value.rs",
            "line=12", "column=7", "expected=", "actual=", "update=",
        ):
            self.assertIn(field, rendered)

    def test_canonical_encoding_mutation_fails(self):
        canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        canonical["primitive_encoding"]["byte_order"] = "native-endian"
        self.assertTrue(CHECKER.canonical_encoding_failures(canonical, Path("canonical.json")))

    def test_canonical_schema_tag_mutation_fails(self):
        canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        canonical["schema_encoding"]["tags"]["Option"] = 99
        self.assertTrue(
            CHECKER.canonical_encoding_failures(canonical, Path("canonical.json"))
        )

    def test_complex_declared_keyable_fails(self):
        canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        keyability = canonical["schema_encoding"]["keyability"]
        keyability["not_keyable"].remove("Complex")
        keyability["always_keyable"].append("Complex")
        self.assertIn(
            "C0-KEY-SEMANTICS",
            {
                item.contract_id
                for item in CHECKER.canonical_encoding_failures(
                    canonical, Path("canonical.json")
                )
            },
        )

    def test_wrong_float_nan_bits_fail(self):
        for width, invalid in (("F32_NaN", "0x7f800001"), ("F64_NaN", "0x7ff0000000000001")):
            with self.subTest(width=width):
                canonical = CHECKER.load_json(
                    CONTRACTS / "canonical-encoding-v1.json"
                )
                canonical["key_encoding"]["float_normalization"][width] = invalid
                self.assertIn(
                    "C0-KEY-SEMANTICS",
                    {
                        item.contract_id
                        for item in CHECKER.canonical_encoding_failures(
                            canonical, Path("canonical.json")
                        )
                    },
                )

    def test_changed_value_hash_domain_fails(self):
        canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        canonical["hashes"]["ValueHash"]["domain_separator_utf8"] = "changed\0"
        self.assertIn(
            "C0-CANONICAL-PAYLOAD-ENCODING",
            {
                item.contract_id
                for item in CHECKER.canonical_encoding_failures(
                    canonical, Path("canonical.json")
                )
            },
        )

    def test_changed_schema_key_domain_fails(self):
        canonical = CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json")
        canonical["hashes"]["SchemaKey"]["domain_separator_utf8"] = "changed\0"
        self.assertIn(
            "C0-CANONICAL-SCHEMA-ENCODING",
            {
                item.contract_id
                for item in CHECKER.canonical_encoding_failures(
                    canonical, Path("canonical.json")
                )
            },
        )

    def test_every_critical_canonical_encoding_rule_is_enforced(self):
        mutations = (
            (
                "unknown-schema-tag",
                lambda value: value["schema_encoding"]["tags"].update({"Unknown": 255}),
            ),
            (
                "index-width",
                lambda value: value["primitive_encoding"]["index"].update(
                    {"semantic_type": "usize"}
                ),
            ),
            (
                "dimension-framing",
                lambda value: value["dimension_parameters"].update({"frame": []}),
            ),
            (
                "duplicate-record-field-rule",
                lambda value: value["schema_encoding"].update(
                    {"record_and_table_names": "duplicates-allowed"}
                ),
            ),
            (
                "recursive-schema",
                lambda value: value["schema_encoding"].update(
                    {"recursive_error": "recursive-schemas-supported"}
                ),
            ),
            (
                "nominal-domain",
                lambda value: value["hashes"]["NominalKey"].update(
                    {"domain_separator_utf8": "changed\0"}
                ),
            ),
            (
                "shape-frame",
                lambda value: value["shape_encoding"].update({"frame": []}),
            ),
            (
                "dimension-overflow",
                lambda value: value["dimension_expression_encoding"].update(
                    {"constant_overflow": "wrapping-u64"}
                ),
            ),
        )
        for name, mutate in mutations:
            with self.subTest(name=name):
                canonical = CHECKER.load_json(
                    CONTRACTS / "canonical-encoding-v1.json"
                )
                mutate(canonical)
                self.assertIn(
                    "C0-CANONICAL-SCHEMA-ENCODING",
                    {
                        item.contract_id
                        for item in CHECKER.canonical_encoding_failures(
                            canonical, Path("canonical.json")
                        )
                    },
                )

    def test_dimension_constant_fold_overflow_is_rejected(self):
        for expression in (
            {
                "kind": "Add",
                "operands": [
                    {"kind": "Constant", "value": (1 << 64) - 1},
                    {"kind": "Constant", "value": 1},
                ],
            },
            {
                "kind": "Multiply",
                "operands": [
                    {"kind": "Constant", "value": 1 << 63},
                    {"kind": "Constant", "value": 2},
                ],
            },
        ):
            with self.subTest(kind=expression["kind"]):
                with self.assertRaisesRegex(ValueError, "DimensionOverflowV1"):
                    CHECKER.CANONICAL_REFERENCE.dimension_expression(expression)

    def test_missing_payload_or_value_hash_fails(self):
        for field in ("payload_hex", "value_hash_hex"):
            with self.subTest(field=field):
                vectors = CHECKER.load_json(
                    CONTRACTS / "canonical-encoding-v1-vectors.json"
                )
                del vectors["value_vectors"][0]["expected"][field]
                self.assertIn(
                    "C0-CANONICAL-PAYLOAD-ENCODING",
                    {
                        item.contract_id
                        for item in CHECKER.canonical_vector_failures(
                            vectors, Path("vectors.json")
                        )
                    },
                )

    def test_duplicate_canonical_map_or_set_keys_cannot_be_allowed(self):
        vectors = CHECKER.load_json(
            CONTRACTS / "canonical-encoding-v1-vectors.json"
        )
        duplicate = next(
            row
            for row in vectors["key_vectors"]
            if row["id"] == "duplicate-canonical-set-keys"
        )
        duplicate["expected"] = {"allowed": True}
        self.assertIn(
            "C0-KEY-SEMANTICS",
            {
                item.contract_id
                for item in CHECKER.canonical_vector_failures(
                    vectors, Path("vectors.json")
                )
            },
        )

    def test_shallow_nested_add_normalization_fails(self):
        vectors = CHECKER.load_json(
            CONTRACTS / "canonical-encoding-v1-vectors.json"
        )
        nested = next(
            row for row in vectors["dimension_vectors"] if row["id"] == "nested-add"
        )
        nested["expected"]["normalized_hex"] = (
            CHECKER.CANONICAL_REFERENCE.encode_normalized_dimension(
                nested["input"]["expression"]
            ).hex()
        )
        self.assertIn(
            "C0-DIMENSION-NORMALIZATION",
            {
                item.contract_id
                for item in CHECKER.canonical_vector_failures(
                    vectors, Path("vectors.json")
                )
            },
        )

    def test_dimension_overflow_or_unknown_parameter_cannot_be_accepted(self):
        for identifier in ("add-overflow", "multiply-overflow", "unknown-parameter"):
            with self.subTest(identifier=identifier):
                vectors = CHECKER.load_json(
                    CONTRACTS / "canonical-encoding-v1-vectors.json"
                )
                row = next(
                    item
                    for item in vectors["dimension_vectors"]
                    if item["id"] == identifier
                )
                row["expected"] = {"normalized_hex": ""}
                self.assertIn(
                    "C0-DIMENSION-NORMALIZATION",
                    {
                        item.contract_id
                        for item in CHECKER.canonical_vector_failures(
                            vectors, Path("vectors.json")
                        )
                    },
                )

    def test_omitted_workspace_package_fails(self):
        live = {
            "workspace_packages": [
                {"name": "one", "manifest": "one/Cargo.toml"},
                {"name": "two", "manifest": "two/Cargo.toml"},
            ]
        }
        inventory = {"workspace_packages": live["workspace_packages"][:1]}
        self.assertIn(
            "C0-WORKSPACE-SOURCE-COVERAGE",
            {
                item.contract_id
                for item in CHECKER.workspace_source_coverage_failures(
                    inventory, live, Path("inventory.json")
                )
            },
        )

    def test_live_matrix_value_rejection_fails(self):
        migration = CHECKER.load_json(CONTRACTS / "migration.json")
        row = next(
            item
            for item in migration["use_classifications"]
            if item["enum"] == "LegacyValue"
            and item["variant"] == "MatrixValue"
            and item["target"] == "legacy-matrix-value-adapter"
        )
        row["target"] = "heterogeneous-matrix-rejected"
        self.assertIn(
            "C0-MATRIX-VALUE-CLASSIFICATION",
            {
                item.contract_id
                for item in CHECKER.matrix_value_classification_failures(
                    migration, Path("migration.json")
                )
            },
        )

    def test_changed_golden_vector_fails(self):
        vectors = CHECKER.load_json(
            CONTRACTS / "canonical-encoding-v1-vectors.json"
        )
        vectors["value_vectors"][0]["expected"]["schema_hex"] = "00"
        self.assertIn(
            "C0-CANONICAL-PAYLOAD-ENCODING",
            {
                item.contract_id
                for item in CHECKER.canonical_vector_failures(
                    vectors, Path("vectors.json")
                )
            },
        )

    def test_encoder_and_vector_cannot_drift_together(self):
        vectors = CHECKER.load_json(
            CONTRACTS / "canonical-encoding-v1-vectors.json"
        )
        vectors["value_vectors"][0]["input"]["value"] = False
        vectors["value_vectors"][0]["expected"] = (
            CHECKER.CANONICAL_REFERENCE.reproduce_value(vectors["value_vectors"][0])
        )
        self.assertIn(
            "C0-CANONICAL-VECTOR-FREEZE",
            {
                item.contract_id
                for item in CHECKER.canonical_vector_failures(
                    vectors,
                    Path("vectors.json"),
                    CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json"),
                )
            },
        )

    def test_invalid_vector_input_and_expected_error_cannot_drift_together(self):
        vectors = CHECKER.load_json(
            CONTRACTS / "canonical-encoding-v1-vectors.json"
        )
        vector = next(
            row
            for row in vectors["invalid_value_vectors"]
            if row["id"] == "tuple-too-short"
        )
        vector["input"]["value"] = []
        vector["expected"] = {"error": "AggregateArityMismatchV1"}
        self.assertIn(
            "C0-CANONICAL-VECTOR-FREEZE",
            {
                item.contract_id
                for item in CHECKER.canonical_vector_failures(
                    vectors,
                    Path("vectors.json"),
                    CHECKER.load_json(CONTRACTS / "canonical-encoding-v1.json"),
                )
            },
        )

    def test_malformed_tuple_key_comparison_cannot_ignore_a_suffix(self):
        schema = {
            "kind": "Tuple",
            "elements": [{"kind": "Bool"}, {"kind": "Bool"}],
        }
        with self.assertRaisesRegex(
            CHECKER.CANONICAL_REFERENCE.EncodingError,
            "AggregateArityMismatchV1",
        ):
            CHECKER.CANONICAL_REFERENCE.compare_keys(
                schema, [True], [True, False]
            )

    def test_cyclic_in_memory_schema_is_rejected(self):
        schema = {"kind": "Option"}
        schema["element"] = schema
        with self.assertRaisesRegex(
            CHECKER.CANONICAL_REFERENCE.EncodingError,
            "RecursiveSchemaUnsupportedV1",
        ):
            CHECKER.CANONICAL_REFERENCE.schema_bytes(schema, [])

    def test_non_keyable_set_elements_and_map_keys_are_rejected(self):
        schemas = (
            {
                "kind": "Set",
                "element": {"kind": "Complex", "component_bit_width": 64},
            },
            {
                "kind": "Map",
                "key": {"kind": "Complex", "component_bit_width": 64},
                "value": {"kind": "Bool"},
            },
        )
        for schema in schemas:
            with self.subTest(kind=schema["kind"]):
                with self.assertRaisesRegex(
                    CHECKER.CANONICAL_REFERENCE.EncodingError,
                    "SchemaNotKeyableV1",
                ):
                    CHECKER.CANONICAL_REFERENCE.schema_bytes(schema, [])

    def test_map_value_payload_receives_shape_parameters(self):
        schema = {
            "kind": "Map",
            "cardinality": {"kind": "Constant", "value": 1},
            "key": {"kind": "UnsignedInteger", "bit_width": 8},
            "value": {
                "kind": "Matrix",
                "element": {"kind": "Bool"},
                "dimensions": [{"kind": "Parameter", "ordinal": 0}],
            },
        }
        payload = CHECKER.CANONICAL_REFERENCE.canonical_payload(
            schema, [[1, [True, False]]], [2]
        )
        self.assertTrue(payload)

    def test_matrix_and_table_dimension_overflow_is_rejected(self):
        matrix = {
            "kind": "Matrix",
            "element": {"kind": "Bool"},
            "dimensions": [
                {"kind": "Constant", "value": (1 << 64) - 1},
                {"kind": "Constant", "value": 2},
            ],
        }
        table = {
            "kind": "Table",
            "columns": [{"name": "x", "schema": {"kind": "Bool"}}],
            "row_count": {
                "kind": "Add",
                "operands": [
                    {"kind": "Constant", "value": (1 << 64) - 1},
                    {"kind": "Constant", "value": 1},
                ],
            },
        }
        for schema, value in ((matrix, []), (table, {"x": []})):
            with self.subTest(kind=schema["kind"]):
                with self.assertRaisesRegex(
                    CHECKER.CANONICAL_REFERENCE.EncodingError,
                    "DimensionOverflowV1",
                ):
                    CHECKER.CANONICAL_REFERENCE.canonical_payload(
                        schema, value, []
                    )


class FinalQualificationFindingTests(unittest.TestCase):
    def test_zero_findings_succeeds(self):
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            self.assertEqual(CHECKER.report_result([]), 0)
        self.assertIn("value-system contract passed", stdout.getvalue())

    def test_stale_performance_evidence_is_advisory(self):
        finding = CHECKER.failure(
            "C0-GATE-B-EVIDENCE-STALE",
            "finding",
            "contract.json",
            "expected",
            "actual",
            "update",
        )
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            self.assertEqual(CHECKER.report_result([finding]), 0)
        self.assertIn("value-system contract advisories", stderr.getvalue())

    def test_semantic_finding_fails(self):
        finding = CHECKER.failure(
            "C0-KIND-SCHEME-SEPARATION",
            "finding",
            "contract.json",
            "expected",
            "actual",
            "update",
        )
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            self.assertEqual(CHECKER.report_result([finding]), 1)
        self.assertIn("value-system contract failed", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
