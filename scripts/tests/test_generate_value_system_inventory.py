import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "generate-value-system-inventory.py"
FIXTURE_TEMPLATES = Path(__file__).resolve().parent / "fixtures/value-system"
SPEC = importlib.util.spec_from_file_location("generate_value_system_inventory", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


def materialize_rust_templates(group, destination):
    written = []
    for template in sorted((FIXTURE_TEMPLATES / group).rglob("*.rs.txt")):
        relative = template.relative_to(FIXTURE_TEMPLATES / group)
        target = destination / relative.with_suffix("")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(template.read_text(encoding="utf-8"), encoding="utf-8")
        written.append(target)
    return written


VALUE_SOURCE = """
pub enum ValueKind {
    Any,
    Empty,
    Matrix(Box<ValueKind>, Vec<usize>),
}

pub enum Value {
    #[cfg(all(feature = "matrix", feature = "f64"))]
    MatrixF64(Matrix<f64>),
    MutableReference(MutableReference),
    Typed(Box<Value>, ValueKind),
    Empty,
}
"""

KIND_SOURCE = """
pub enum Kind {
    Any,
    Empty,
    Reference(Box<Kind>),
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


class ValueSystemInventoryGeneratorTests(unittest.TestCase):
    def repository(self, extra=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        files = {
            "src/core/src/value.rs": VALUE_SOURCE,
            "src/core/src/kind.rs": KIND_SOURCE,
            "src/core/src/lib.rs": "mod value;\nmod kind;\n",
            "src/core/src/nodes.rs": NODES_SOURCE,
            "src/core/src/function/argument.rs": ARGUMENT_SOURCE,
            "src/core/src/function/signature.rs": SIGNATURE_SOURCE,
        }
        files.update(extra or {})
        for relative, source in files.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source, encoding="utf-8")
        return root

    def generate(self, root):
        return GENERATOR.generate(
            root, "f" * 40, target_roots=[root / "src/core/src/lib.rs"]
        )

    @staticmethod
    def uses(inventory, path=None, enum_name=None, variant=None):
        return [
            row
            for row in inventory["variant_uses"]
            if (path is None or row["path"] == path)
            and (enum_name is None or row["enum"] == enum_name)
            and (variant is None or row["variant"] == variant)
        ]

    def test_balanced_parser_handles_cfg_and_multiline_payload(self):
        variants = GENERATOR.parse_enum(VALUE_SOURCE, "Value", value=True)
        matrix = variants[0]
        self.assertEqual(matrix["name"], "MatrixF64")
        self.assertEqual(matrix["payload_type"], "Matrix<f64>")
        self.assertEqual(matrix["cfg"], 'all(feature = "matrix",feature = "f64")')

    def test_generator_is_byte_deterministic(self):
        root = self.repository()
        first = GENERATOR.render(self.generate(root))
        second = GENERATOR.render(self.generate(root))
        self.assertEqual(first, second)
        self.assertEqual(json.loads(first), json.loads(second))

    def test_complete_scanner_module_digest_covers_every_scanner_layer(self):
        scanner_path = Path(GENERATOR.LEGACY_SCANNER.__file__)
        source = scanner_path.read_text(encoding="utf-8")
        original = GENERATOR.LEGACY_SCANNER.scanner_module_sha256(scanner_path)
        markers = (
            "TOKEN_PATTERN = re.compile",
            "def mask_non_code",
            "def canonical_identifier",
            "LEGACY_ALIAS_SPECS =",
            "def type_alias_declarations",
            "HIGH_RISK_PATTERNS =",
            "def ref_method_definition_uses",
            "def ref_ufcs_uses",
            "def ref_instance_use_sites",
            "def grouped_uses",
            "class TransparentTypeResolver",
        )
        for marker in markers:
            with self.subTest(marker=marker):
                self.assertIn(marker, source)
                temporary = tempfile.TemporaryDirectory()
                self.addCleanup(temporary.cleanup)
                changed = Path(temporary.name) / "scanner.py"
                changed.write_text(
                    source.replace(marker, marker + " # drift", 1),
                    encoding="utf-8",
                )
                self.assertNotEqual(
                    GENERATOR.LEGACY_SCANNER.scanner_module_sha256(changed),
                    original,
                )

    def test_c0_rust_templates_are_materialized_before_scanning(self):
        root = self.repository()
        written = materialize_rust_templates(
            "new-legacy-use", root / "src/core/src/c0-fixtures"
        )
        self.assertTrue(written)
        self.assertTrue(all(path.suffix == ".rs" for path in written))
        inventory = self.generate(root)
        self.assertTrue(inventory["high_risk_api_uses"]["valref-alias"])
        self.assertTrue(
            inventory["high_risk_api_uses"]["value-mutable-reference"]
        )

    def test_every_c0_rust_template_materializes_as_rust_source(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        destination = Path(temporary.name)
        templates = sorted(FIXTURE_TEMPLATES.rglob("*.rs.txt"))
        written = []
        for group in sorted({path.parent.name for path in templates}):
            written.extend(materialize_rust_templates(group, destination / group))
        self.assertEqual(len(written), len(templates))
        self.assertTrue(all(path.suffix == ".rs" for path in written))

    def test_all_three_enum_uses_are_inventoried(self):
        root = self.repository(
            {
                "src/core/src/live.rs": (
                    "fn live() { let _ = Value::Empty; let _ = ValueKind::Any; "
                    "let _ = crate::kind::Kind::Reference("
                    "Box::new(crate::kind::Kind::Any)); }\n"
                )
            }
        )
        names = {(row["enum"], row["variant"]) for row in self.generate(root)["variant_uses"]}
        self.assertTrue({("Value", "Empty"), ("ValueKind", "Any"), ("Kind", "Reference")} <= names)

    def test_same_variant_twice_on_one_line_has_distinct_columns(self):
        root = self.repository(
            {"src/core/src/live.rs": "fn live() { let _ = (Value::Empty, Value::Empty); }\n"}
        )
        rows = self.uses(self.generate(root), "src/core/src/live.rs", "Value", "Empty")
        self.assertEqual(len(rows), 2)
        self.assertEqual(len({row["column"] for row in rows}), 2)

    def test_qualified_use_accepts_whitespace_and_comments(self):
        root = self.repository(
            {
                "src/core/src/live.rs": (
                    "fn live() { let _ = Value /*a*/ :: /*b*/ Empty; "
                    "let _ = crate::kind::Kind \n :: \n Reference; }\n"
                )
            }
        )
        rows = self.uses(self.generate(root), "src/core/src/live.rs")
        self.assertEqual({(row["enum"], row["variant"]) for row in rows}, {("Value", "Empty"), ("Kind", "Reference")})

    def qualification(self, statement):
        source = statement + "\nfn live() {}\n"
        tokens = GENERATOR.rust_tokens(source)
        return GENERATOR.qualification_violations(
            "src/core/src/live.rs",
            tokens,
            {
                "Value": {"Empty", "Typed", "MatrixValue"},
                "ValueKind": {"Any", "Empty"},
                "Kind": {"Any", "Empty", "Reference"},
            },
        )

    def test_value_glob_import_fails(self):
        self.assertEqual(self.qualification("use crate::Value::*;")[0]["kind"], "glob-import")

    def test_value_kind_glob_import_fails(self):
        self.assertEqual(self.qualification("use crate::ValueKind::*;")[0]["enum"], "ValueKind")

    def test_kind_glob_import_fails(self):
        self.assertEqual(self.qualification("use mech_core::Kind::*;")[0]["enum"], "Kind")

    def test_grouped_variant_import_fails(self):
        self.assertEqual(self.qualification("use crate::Value::{Empty, Typed};")[0]["kind"], "grouped-variant-import")

    def test_single_variant_import_fails(self):
        self.assertEqual(self.qualification("use crate::Value::Empty;")[0]["kind"], "single-variant-import")

    def test_variant_alias_fails(self):
        self.assertEqual(self.qualification("use crate::Value::Empty as Missing;")[0]["kind"], "variant-alias")

    def test_enum_alias_fails(self):
        self.assertEqual(self.qualification("use crate::Value as LegacyValue;")[0]["kind"], "enum-alias")

    def test_type_alias_fails(self):
        self.assertEqual(self.qualification("type LegacyValue = crate::Value;")[0]["kind"], "type-alias")

    def test_generic_indirect_type_alias_fails(self):
        violations = self.qualification(
            "type Identity<T> = T; type V = Identity<Value>; fn live() { let _ = V::Empty; }"
        )
        self.assertIn("type-alias", {row["kind"] for row in violations})

    def test_transitive_type_alias_fails(self):
        violations = self.qualification(
            "type First = crate::Value; type Second = First; "
            "fn live() { let _ = Second::Empty; }"
        )
        self.assertEqual(
            [row["enum"] for row in violations if row["kind"] == "type-alias"],
            ["Value", "Value"],
        )

    def test_container_type_alias_passes(self):
        self.assertEqual(
            self.qualification("type ValueTable = HashMap<u64, Value>;"),
            [],
        )

    def test_parenthesized_and_identity_wrapped_audited_aliases_fail(self):
        for source in (
            "type V = (crate::Value); let _ = V::Empty;",
            "type V = self::Value; let _ = V::Empty;",
            "type Identity<T> = T; type V = Identity<(crate::Value)>; let _ = V::Empty;",
        ):
            with self.subTest(source=source):
                self.assertIn(
                    "type-alias", {row["kind"] for row in self.qualification(source)}
                )

    def test_parenthesized_and_transitive_ref_aliases_fail(self):
        for source in (
            "type R<T> = (Ref<T>); let _ = R::<T>::id(&value);",
            "type Identity<T> = T; type R<T> = Identity<(Ref<T>)>; let _ = R::<T>::as_ptr(&value);",
            "type First<T> = Ref<T>; type Second<T> = First<T>; let _ = Second::<T>::addr(&value);",
        ):
            with self.subTest(source=source):
                self.assertIn(
                    "ref-alias", {row["kind"] for row in self.qualification(source)}
                )

    def test_type_alias_cycle_fails_conservatively(self):
        violations = self.qualification("type A = B; type B = A;")
        self.assertIn("type-alias-cycle", {row["kind"] for row in violations})

    def test_raw_audited_aliases_fail(self):
        for source in (
            "use crate::r#Value as V;",
            "type V = crate::r#Value;",
            "use crate::r#ValueKind as VK;",
            "type VK = crate::r#ValueKind;",
            "use crate::kind::r#Kind as K;",
            "type K = crate::kind::r#Kind;",
        ):
            with self.subTest(source=source):
                self.assertIn(
                    "raw-audited-alias",
                    {row["kind"] for row in self.qualification(source)},
                )

    def test_ref_aliases_fail(self):
        for source in (
            "use crate::Ref as R;",
            "pub use crate::Ref as R;",
            "use crate::{Ref as R};",
            "use crate::r#Ref as R;",
            "type R<T> = Ref<T>;",
            "pub type R<T> = crate::Ref<T>;",
            "type Identity<T> = T; type R<T> = Identity<Ref<T>>;",
        ):
            with self.subTest(source=source):
                self.assertIn(
                    "ref-alias", {row["kind"] for row in self.qualification(source)}
                )

    def test_frozen_ref_alias_definitions_remain_approved(self):
        source = "pub type MutableReference = Ref<Value>; pub type ValRef = Ref<Value>;"
        violations = GENERATOR.qualification_violations(
            "src/core/src/types/mod.rs", GENERATOR.rust_tokens(source)
        )
        self.assertNotIn("ref-alias", {row["kind"] for row in violations})

        compatibility = (
            ("src/core/src/program/symbol_table.rs", "pub type SymbolTableRef = Ref<SymbolTable>;"),
        )
        for path, declaration in compatibility:
            with self.subTest(path=path):
                violations = GENERATOR.qualification_violations(
                    path, GENERATOR.rust_tokens(declaration)
                )
                self.assertNotIn("ref-alias", {row["kind"] for row in violations})

    def test_semantic_and_syntax_kind_paths_are_distinguished(self):
        variants = {
            "Value": {"Empty"},
            "ValueKind": {"Any"},
            "Kind": {"Any"},
        }
        cases = (
            ("fn f() { mech_core::kind::Kind::Any; }", 1),
            ("fn f() { mech_core::Kind::Any; }", 1),
            ("fn f() { mech_core::nodes::Kind::Any; }", 0),
            ("use mech_core::nodes::Kind as NodeKind; fn f() { NodeKind::Any; }", 0),
            ("use mech_core::kind::Kind; fn f() { Kind::Any; }", 1),
            ("use crate::{kind::Kind}; fn f() { Kind::Any; }", 1),
            ("use mech_core::nodes::Kind; fn f() { Kind::Any; }", 0),
        )
        for source, expected in cases:
            with self.subTest(source=source):
                tokens = GENERATOR.rust_tokens(source)
                rows = GENERATOR.variant_uses("new/src/lib.rs", tokens, variants)
                self.assertEqual(len(rows), expected)

    def test_crate_glob_resolves_exact_root_kind_bindings(self):
        root = self.repository(
            {
                "src/core/Cargo.toml": (
                    '[package]\nname = "fixture"\nversion = "0.0.0"\n'
                ),
                "src/core/src/lib.rs": (
                    "mod value; mod kind; mod nodes; mod live; "
                    "use crate::kind::Kind; "
                    "use crate::nodes::Kind as NodeKind;\n"
                ),
                "src/core/src/live.rs": (
                    "use crate::*; fn live() { "
                    "let _ = Kind::Any; let _ = NodeKind::Any; }\n"
                ),
            }
        )
        rows = self.uses(
            self.generate(root), "src/core/src/live.rs", "Kind", "Any"
        )
        self.assertEqual(len(rows), 1)

    def test_ambiguous_bare_kind_fails(self):
        violations = self.qualification("use mech_core::*; fn f() { Kind::Any; }")
        self.assertIn(
            "kind-qualifier-ambiguous", {row["kind"] for row in violations}
        )

    def test_kind_resolution_is_lexically_scoped(self):
        variants = {"Value": {"Empty"}, "ValueKind": {"Any"}, "Kind": {"Any"}}
        source = (
            "use mech_core::kind::Kind;\n"
            "mod syntax { enum Kind { Any } fn local() { let _ = Kind::Any; } }\n"
            "mod sibling { use mech_core::nodes::Kind; fn local() { let _ = Kind::Any; } }\n"
            "fn parent() { let _ = Kind::Any; }\n"
        )
        rows = GENERATOR.variant_uses(
            "src/core/src/live.rs", GENERATOR.rust_tokens(source), variants
        )
        self.assertEqual(len(rows), 1)

    def test_nested_kind_declaration_cannot_hide_root_wildcard_ambiguity(self):
        source = (
            "use mech_core::*; mod syntax { enum Kind { Any } } "
            "fn live() { let _ = Kind::Any; }"
        )
        self.assertIn(
            "kind-qualifier-ambiguous",
            {row["kind"] for row in self.qualification(source)},
        )

    def test_self_qualified_variant_fails(self):
        for source, kind in (
            ("impl Value { fn empty() -> Self { Self::Empty } }", "self-qualified-variant"),
            ("impl Display for Value { fn f() { <Self>::Empty; } }", "self-type-qualified-variant"),
        ):
            with self.subTest(source=source):
                self.assertIn(kind, {row["kind"] for row in self.qualification(source)})

    def test_self_qualified_variant_in_unrelated_impl_passes(self):
        self.assertEqual(
            self.qualification("impl Other { fn empty() { Self::Empty; <Self>::Empty; } }"),
            [],
        )

    def test_type_qualified_variant_fails(self):
        for source in (
            "fn live() { let _ = <Value>::Empty; }",
            "fn live() { let _ = <crate::Value>::Empty; }",
            "fn live() { let _ = <Value as Trait>::Empty; }",
        ):
            with self.subTest(source=source):
                self.assertEqual(
                    self.qualification(source)[0]["kind"],
                    "type-qualified-variant",
                )

    def test_generic_qualified_variant_fails(self):
        self.assertEqual(
            self.qualification("fn live<T>() { let _ = Value::<T>::Empty; }")[0]["kind"],
            "generic-qualified-variant",
        )

    def test_canonical_qualified_variants_pass(self):
        for source in (
            "fn live() { let _ = Value::Empty; }",
            "fn live() { let _ = crate::Value::Empty; }",
            "fn live() { let _ = Value /* comment */ :: Empty; }",
        ):
            with self.subTest(source=source):
                self.assertEqual(self.qualification(source), [])

    def test_raw_identifier_variant_fails(self):
        for source in (
            "fn live() { let _ = Value::r#Empty; }",
            "fn live() { let _ = r#Value::Empty; }",
        ):
            with self.subTest(source=source):
                self.assertEqual(
                    self.qualification(source)[0]["kind"],
                    "raw-identifier-qualified-variant",
                )

    def audited_paths(self, root):
        return {
            path.relative_to(root).as_posix()
            for path in GENERATOR.production_files(
                root, target_roots=[root / "src/core/src/lib.rs"]
            )
        }

    def test_test_only_external_module_excluded(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": "#[cfg(test)] mod tests;\n",
                "src/core/src/tests.rs": "fn ignored() { let _ = Value::Empty; }\n",
            }
        )
        self.assertNotIn("src/core/src/tests.rs", self.audited_paths(root))

    def test_all_test_feature_external_module_excluded(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": '#[cfg(all(test, feature = "x"))] mod tests;\n',
                "src/core/src/tests.rs": "fn ignored() {}\n",
            }
        )
        self.assertNotIn("src/core/src/tests.rs", self.audited_paths(root))

    def test_mixed_test_runtime_cfg_is_audited(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": '#[cfg(any(test, feature = "runtime"))] mod mixed;\n',
                "src/core/src/mixed.rs": "fn live() {}\n",
            }
        )
        self.assertIn("src/core/src/mixed.rs", self.audited_paths(root))

    def test_not_test_module_is_audited(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": "#[cfg(not(test))] mod live;\n",
                "src/core/src/live.rs": "fn live() {}\n",
            }
        )
        self.assertIn("src/core/src/live.rs", self.audited_paths(root))

    def test_dual_reachable_file_is_audited(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": "#[cfg(test)] mod shared;\n#[cfg(not(test))] mod shared;\n",
                "src/core/src/shared.rs": "fn live() {}\n",
            }
        )
        self.assertIn("src/core/src/shared.rs", self.audited_paths(root))

    def test_explicit_path_module_is_audited(self):
        root = self.repository(
            {
                "src/core/src/lib.rs": '#[path = "odd/location.rs"] mod live;\n',
                "src/core/src/odd/location.rs": "fn live() {}\n",
            }
        )
        self.assertIn("src/core/src/odd/location.rs", self.audited_paths(root))

    def test_production_path_into_named_fixture_directories_is_audited(self):
        for directory in ("scripts/tests/fixtures", "tests/fixtures"):
            with self.subTest(directory=directory):
                root = self.repository(
                    {
                        "src/core/src/lib.rs": (
                            f'#[path = "../../../{directory}/live.rs"] mod live;\n'
                        ),
                        f"{directory}/live.rs": "fn live() { let _ = Value::Empty; }\n",
                    }
                )
                self.assertIn(f"{directory}/live.rs", self.audited_paths(root))

    def test_production_path_escaping_repository_fails(self):
        root = self.repository()
        outside = root.parent / f"{root.name}-outside.rs"
        outside.write_text("fn outside() {}\n", encoding="utf-8")
        self.addCleanup(lambda: outside.unlink(missing_ok=True))
        (root / "src/core/src/lib.rs").write_text(
            f'#[path = "../../../../{outside.name}"] mod outside;\n', encoding="utf-8"
        )
        with self.assertRaises(GENERATOR.AuxiliaryFixtureError):
            GENERATOR.production_files(
                root, target_roots=[root / "src/core/src/lib.rs"]
            )

    def test_unreferenced_tests_filename_is_audited(self):
        root = self.repository({"src/core/src/tests.rs": "fn production() {}\n"})
        self.assertIn("src/core/src/tests.rs", self.audited_paths(root))

    def test_cargo_integration_test_target_is_excluded(self):
        root = self.repository(
            {
                "Cargo.toml": '[workspace]\nmembers = ["src/core"]\nresolver = "2"\n',
                "src/core/Cargo.toml": '[package]\nname = "fixture"\nversion = "0.0.0"\n',
                "src/core/tests/contract.rs": "fn ignored() {}\n",
            }
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertNotIn("src/core/tests/contract.rs", paths)

    def workspace_repository(self):
        return self.repository(
            {
                "Cargo.toml": (
                    '[workspace]\nmembers = ["src/core", "new-top/package"]\n'
                    'resolver = "2"\n'
                ),
                "src/core/Cargo.toml": (
                    '[package]\nname = "core-fixture"\nversion = "0.0.0"\n'
                ),
                "new-top/package/Cargo.toml": (
                    '[package]\nname = "new-package"\nversion = "0.0.0"\n'
                ),
                "new-top/package/src/lib.rs": (
                    "mod unreferenced; fn live() { let _ = Value::Empty; }\n"
                ),
                "new-top/package/src/unreferenced.rs": (
                    "fn extra() { let _ = ValueKind::Any; }\n"
                ),
                "new-top/package/build.rs": "fn main() { let _ = Value::Empty; }\n",
                "new-top/package/target/generated.rs": (
                    "fn generated() { let _ = Value::Empty; }\n"
                ),
            }
        )

    def test_workspace_package_outside_fixed_roots_is_audited(self):
        root = self.workspace_repository()
        inventory = GENERATOR.generate(root)
        paths = {row["path"] for row in inventory["variant_uses"]}
        self.assertIn("new-top/package/src/lib.rs", paths)
        self.assertIn("new-top/package/src/unreferenced.rs", paths)
        self.assertIn("new-top/package/build.rs", paths)
        self.assertNotIn("new-top/package/target/generated.rs", paths)
        packages = {row["name"]: row for row in inventory["workspace_packages"]}
        self.assertEqual(packages["new-package"]["rust_file_count"], 3)

    def test_cargo_metadata_failure_is_fatal_without_injected_roots(self):
        root = self.repository()
        with self.assertRaises(GENERATOR.CargoMetadataError):
            GENERATOR.production_files(root)

    def test_unmatched_nested_test_fixture_remains_audited(self):
        root = self.repository(
            {
                "Cargo.toml": '[workspace]\nmembers = ["src/core"]\nresolver = "2"\n',
                "src/core/Cargo.toml": '[package]\nname = "fixture"\nversion = "0.0.0"\n',
                "src/core/tests/contract.rs": "fn test_root() {}\n",
                "src/core/tests/ui/compile_fail.rs": "fn fixture() { let _ = Value::Empty; }\n",
            }
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertIn("src/core/tests/ui/compile_fail.rs", paths)

    def trybuild_repository(self, driver, fixtures):
        return self.repository(
            {
                "Cargo.toml": '[workspace]\nmembers = ["src/core"]\nresolver = "2"\n',
                "src/core/Cargo.toml": '[package]\nname = "fixture"\nversion = "0.0.0"\n',
                "src/core/tests/contract.rs": driver,
                **fixtures,
            }
        )

    def test_literal_compile_fail_and_pass_fixtures_are_excluded(self):
        root = self.trybuild_repository(
            (
                "#[test] fn cases() { let tests = trybuild::TestCases::new(); "
                'tests.compile_fail("tests/ui/fail/*.rs"); '
                'tests.pass("tests/ui/pass/*.rs"); }\n'
            ),
            {
                "src/core/tests/ui/fail/a.rs": "fn fail() {}\n",
                "src/core/tests/ui/pass/a.rs": "fn pass() {}\n",
            },
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertNotIn("src/core/tests/ui/fail/a.rs", paths)
        self.assertNotIn("src/core/tests/ui/pass/a.rs", paths)

    def test_trybuild_call_in_reachable_test_helper_is_discovered(self):
        root = self.trybuild_repository(
            "mod cases;\n",
            {
                "src/core/tests/cases.rs": (
                    "fn cases() { let tests = trybuild::TestCases::new(); "
                    'tests.compile_fail("tests/ui/helper/*.rs"); }\n'
                ),
                "src/core/tests/ui/helper/a.rs": "fn fail() {}\n",
            },
        )
        production, records = GENERATOR.production_inventory(root)
        self.assertNotIn(root / "src/core/tests/ui/helper/a.rs", production)
        self.assertEqual(records[0]["driver"], "src/core/tests/cases.rs")

    def test_raw_string_trybuild_pattern_is_supported(self):
        root = self.trybuild_repository(
            'fn cases() { trybuild::TestCases::new().compile_fail(r#"tests/ui/*.rs"#); }\n',
            {"src/core/tests/ui/a.rs": "fn fail() {}\n"},
        )
        production, records = GENERATOR.production_inventory(root)
        self.assertNotIn(root / "src/core/tests/ui/a.rs", production)
        self.assertEqual(records[0]["pattern"], "tests/ui/*.rs")

    def test_matched_fixture_module_child_is_excluded(self):
        root = self.trybuild_repository(
            'fn cases() { trybuild::TestCases::new().compile_fail("tests/ui/case.rs"); }\n',
            {
                "src/core/tests/ui/case.rs": "mod child;\n",
                "src/core/tests/ui/child.rs": "fn child() {}\n",
            },
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertNotIn("src/core/tests/ui/case.rs", paths)
        self.assertNotIn("src/core/tests/ui/child.rs", paths)

    def test_dynamic_trybuild_pattern_remains_audited(self):
        root = self.trybuild_repository(
            "fn cases(pattern: &str) { let tests = trybuild::TestCases::new(); tests.compile_fail(pattern); }\n",
            {"src/core/tests/ui/dynamic.rs": "fn dynamic() {}\n"},
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertIn("src/core/tests/ui/dynamic.rs", paths)

    def test_unmatched_trybuild_fixture_remains_audited(self):
        root = self.trybuild_repository(
            'fn cases() { trybuild::TestCases::new().compile_fail("tests/ui/matched.rs"); }\n',
            {
                "src/core/tests/ui/matched.rs": "fn matched() {}\n",
                "src/core/tests/ui/unmatched.rs": "fn unmatched() {}\n",
            },
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertNotIn("src/core/tests/ui/matched.rs", paths)
        self.assertIn("src/core/tests/ui/unmatched.rs", paths)

    def test_trybuild_pattern_cannot_escape_package_root(self):
        root = self.trybuild_repository(
            'fn cases() { trybuild::TestCases::new().compile_fail("../outside/*.rs"); }\n',
            {},
        )
        with self.assertRaises(GENERATOR.AuxiliaryFixtureError):
            GENERATOR.production_files(root)

    def test_trybuild_fixture_also_reachable_from_production_is_audited(self):
        root = self.trybuild_repository(
            'fn cases() { trybuild::TestCases::new().compile_fail("tests/ui/shared.rs"); }\n',
            {
                "src/core/src/lib.rs": '#[path = "../tests/ui/shared.rs"] mod shared;\n',
                "src/core/tests/ui/shared.rs": "fn shared() {}\n",
            },
        )
        paths = {path.relative_to(root).as_posix() for path in GENERATOR.production_files(root)}
        self.assertIn("src/core/tests/ui/shared.rs", paths)

    def test_legacy_scanner_is_token_aware(self):
        root = self.repository(
            {
                "src/core/src/live.rs": (
                    'fn live(other: Other) { let _ = "ValRef Value::Typed"; '
                    "other.id(); other.as_ptr(); let _ = Value /*x*/ :: Typed; }\n"
                )
            }
        )
        inventory = self.generate(root)
        self.assertEqual(sum(row["count"] for row in inventory["high_risk_api_uses"]["value-typed-wrapper"]), 1)
        self.assertEqual(inventory["high_risk_api_uses"]["ref-id-ufcs"], [])
        self.assertEqual(inventory["high_risk_api_uses"]["ref-as-ptr-ufcs"], [])

    def test_legacy_scanner_canonicalizes_raw_identifiers(self):
        root = self.repository(
            {
                "src/core/src/live.rs": (
                    "fn live(value: r#ValRef, id: r#ReactiveCellId) {\n"
                    "  let _ = r#Value::r#Typed;\n"
                    "  r#transaction_state_values();\n"
                    "}\n"
                )
            }
        )
        inventory = self.generate(root)
        for identifier in (
            "valref-alias",
            "reactive-cell-id",
            "value-typed-wrapper",
            "transaction-state-values-api",
        ):
            with self.subTest(identifier=identifier):
                self.assertEqual(
                    sum(
                        row["count"]
                        for row in inventory["high_risk_api_uses"][identifier]
                    ),
                    1,
                )

    def test_raw_ref_methods_and_all_high_risk_names_remain_visible(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "fn consume(_: r#ValRef, _: r#MutableReference, _: r#InterpreterRef, "
                    "_: r#ReactiveCellId, _: r#ValueStateJournal) {\n"
                    "  r#transaction_state_values();\n"
                    "  Ref::r#id(&value); Ref::<T>::r#as_ptr(&value);\n"
                    "  <r#Ref<T>>::r#addr(&value); <Ref<T>>::r#as_mut_ptr(&value);\n"
                    "}\n"
                )
            }
        )
        inventory = self.generate(root)
        for identifier in (
            "valref-alias",
            "mutable-reference-alias",
            "reactive-cell-id",
            "value-state-journal",
            "transaction-state-values-api",
            "ref-id-ufcs",
            "ref-as-ptr-ufcs",
            "ref-addr-ufcs",
            "ref-as-mut-ptr-ufcs",
        ):
            with self.subTest(identifier=identifier):
                self.assertTrue(inventory["high_risk_api_uses"][identifier])

    def test_raw_approved_alias_is_inventoried_with_target_drift(self):
        source = "pub type r#ValRef = Ref<Other>;\n"
        tokens = GENERATOR.rust_tokens(source)
        legacy, compatibility = GENERATOR.aliases(
            [
                (
                    "src/core/src/types/mod.rs",
                    source,
                    GENERATOR.mask_non_code(source),
                    tokens,
                )
            ]
        )
        self.assertEqual(compatibility, [])
        self.assertEqual(legacy[0]["raw_name"], "r#ValRef")
        self.assertEqual(legacy[0]["target"], "Ref<Other>")

    def test_ref_pointer_definitions_are_scoped_to_impl_ref(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "struct Ref<T>(T); struct Other;\n"
                    "impl<T> Ref<T> { fn id(&self) -> usize { 0 } "
                    "fn as_ptr(&self) -> *const T { core::ptr::null() } }\n"
                    "impl Other { fn id(&self) -> usize { 0 } }\n"
                )
            }
        )
        inventory = self.generate(root)
        self.assertEqual(
            sum(
                row["count"]
                for row in inventory["high_risk_api_uses"]["ref-id-definition"]
            ),
            1,
        )

    def test_ref_ufcs_scanner_counts_direct_and_generic_qualifiers(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "fn identity<T>(value: Ref<T>) {\n"
                    "  let _ = Ref::id(&value);\n"
                    "  let _ = Ref::<T>::id(&value);\n"
                    "  let _ = crate::Ref::<T>::addr(&value);\n"
                    "  let _ = <Ref<T>>::as_ptr(&value);\n"
                    "  let _ = <crate::Ref<Vec<T>>>::as_mut_ptr(&value);\n"
                    "  let _ = Ref::<Option<Vec<T>>>::addr(&value);\n"
                    "  let _ = Other::<T>::id(&value);\n"
                    "  let _ = <Ref<T> as SomeTrait>::id(&value);\n"
                    "  let _ = value.id();\n"
                    "  let _ = value.r#id();\n"
                    "}\n"
                    "fn unrelated<T>(value: Other<T>) { let _ = value.id(); }\n"
                )
            }
        )
        inventory = self.generate(root)
        self.assertEqual(
            sum(row["count"] for row in inventory["high_risk_api_uses"]["ref-id-ufcs"]),
            4,
        )
        self.assertEqual(
            sum(row["count"] for row in inventory["high_risk_api_uses"]["ref-as-ptr-ufcs"]),
            1,
        )
        self.assertEqual(
            sum(row["count"] for row in inventory["high_risk_api_uses"]["ref-as-mut-ptr-ufcs"]),
            1,
        )
        self.assertEqual(
            sum(row["count"] for row in inventory["high_risk_api_uses"]["ref-addr-ufcs"]),
            2,
        )
        self.assertEqual(
            sum(
                row["count"]
                for row in inventory["high_risk_api_uses"]["ref-as-ptr-definition"]
            ),
            0,
        )

    def test_ref_alias_declarations_cannot_hide_ufcs_calls(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "use crate::Ref as ImportedRef;\n"
                    "type LocalRef<T> = Ref<T>;\n"
                    "fn identity<T>(value: Ref<T>) {\n"
                    "  let _ = ImportedRef::<T>::id(&value);\n"
                    "  let _ = <LocalRef<T>>::id(&value);\n"
                    "}\n"
                )
            }
        )
        source = (root / "src/core/src/identity.rs").read_text(encoding="utf-8")
        violations = GENERATOR.qualification_violations(
            "src/core/src/identity.rs", GENERATOR.rust_tokens(source)
        )
        self.assertEqual(
            sum(row["kind"] == "ref-alias" for row in violations), 2
        )

    def test_indirect_ref_alias_cannot_hide_ufcs_calls(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "type Identity<T> = T;\n"
                    "type Alias<T> = Identity<Ref<T>>;\n"
                    "fn identity<T>(value: Ref<T>) {\n"
                    "  let _ = Alias::<T>::id(&value);\n"
                    "}\n"
                )
            }
        )
        source = (root / "src/core/src/identity.rs").read_text(encoding="utf-8")
        violations = GENERATOR.qualification_violations(
            "src/core/src/identity.rs", GENERATOR.rust_tokens(source)
        )
        self.assertEqual(
            sum(row["kind"] == "ref-alias" for row in violations), 1
        )
        inventory = self.generate(root)
        self.assertEqual(
            sum(
                row["count"]
                for row in inventory["high_risk_api_uses"]["ref-id-ufcs"]
            ),
            1,
        )

    def test_ref_compatibility_alias_parameter_instance_calls_are_counted(self):
        root = self.repository(
            {
                "src/core/src/identity.rs": (
                    "fn identity(value: SymbolTableRef) {\n"
                    "  let _ = value.addr();\n"
                    "}\n"
                )
            }
        )
        inventory = self.generate(root)
        self.assertEqual(
            sum(
                row["count"]
                for row in inventory["high_risk_api_uses"]["ref-addr-ufcs"]
            ),
            1,
        )

    def test_runtime_representation_alias_to_kind_scheme_is_rejected(self):
        root = self.repository(
            {
                "src/core/src/function/signature.rs": SIGNATURE_SOURCE.replace(
                    "pub trait FunctionRuntimeType {}",
                    "pub type FunctionRuntimeType = KindScheme;",
                )
            }
        )
        with self.assertRaisesRegex(ValueError, "source shape changed"):
            GENERATOR.type_contract_sources(root)

    def test_runtime_representation_body_cannot_reference_kind_scheme(self):
        root = self.repository(
            {
                "src/core/src/function/signature.rs": SIGNATURE_SOURCE.replace(
                    "pub enum FunctionValueRepresentation { F64 }",
                    "pub enum FunctionValueRepresentation { F64, Scheme(KindScheme) }",
                )
            }
        )
        with self.assertRaisesRegex(ValueError, "crosses into semantic typing"):
            GENERATOR.type_contract_sources(root)

    def test_kind_scheme_source_field_shape_is_verified(self):
        root = self.repository(
            {
                "src/core/src/nodes.rs": NODES_SOURCE.replace(
                    "pub input: Vec<FunctionArgument>",
                    "pub input: RuntimeFunctionInputs",
                    1,
                )
            }
        )
        with self.assertRaisesRegex(ValueError, "source shape changed"):
            GENERATOR.type_contract_sources(root)


if __name__ == "__main__":
    unittest.main()
