from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r3-type-system.py"
SPEC = importlib.util.spec_from_file_location("check_r3_type_system", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class R3TypeSystemCheckerTests(unittest.TestCase):
    def fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in CHECKER.REQUIRED:
            source = REPOSITORY / relative
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        return root

    @staticmethod
    def write(root: Path, relative: str, source: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")

    def replace(self, root: Path, relative: str, old: str, new: str) -> None:
        path = root / relative
        source = path.read_text(encoding="utf-8")
        self.assertIn(old, source)
        self.write(root, relative, source.replace(old, new, 1))

    def assert_failure(self, root: Path, diagnostic: str) -> None:
        failures = CHECKER.failures(root)
        self.assertTrue(any(diagnostic in item for item in failures), failures)

    def test_01_repository_fixture_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_02_missing_permanent_module_fails(self):
        root = self.fixture()
        (root / "src/core/src/type_system/solver.rs").unlink()
        self.assert_failure(root, "required file is missing")

    def test_03_comments_strings_and_cfg_tests_are_ignored(self):
        root = self.fixture()
        self.write(
            root,
            "src/core/src/type_system/probe.rs",
            '// ValueCell Resident GPU\nconst TEXT: &str = "FunctionRuntimeType";\n',
        )
        self.write(
            root,
            "src/core/src/probe.rs",
            "#[cfg(test)] mod tests { fn old() { scheme_from_signature(); } }\n",
        )
        self.assertEqual(CHECKER.failures(root), [])

    def test_04_storage_bound_type_system_identifier_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/type_system/probe.rs", "struct Bad(ValueCell);\n")
        self.assert_failure(root, "storage-bound type-system identifier ValueCell")

    def test_05_target_specific_type_system_identifier_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/type_system/probe.rs", "struct GpuType;\n")
        self.assert_failure(root, "target-specific")

    def test_06_builtin_ordinal_change_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/type_system/builtin.rs", "    U64 = 3,", "    U64 = 30,")
        self.assert_failure(root, "U64=3")

    def test_07_missing_builtin_predicate_fails(self):
        root = self.fixture()
        self.replace(root, "src/core/src/type_system/builtin.rs", "RangeEndpoint,", "RangeEnd,")
        self.assert_failure(root, "missing RangeEndpoint")

    def test_08_missing_predicate_or_promotion_constraint_fails(self):
        for variant in ("Satisfies", "Promotes"):
            with self.subTest(variant=variant):
                root = self.fixture()
                self.replace(root, "src/core/src/kind_scheme.rs", variant, f"Missing{variant}")
                self.assert_failure(root, f"missing {variant}")

    def test_09_rigid_alias_path_fails(self):
        root = self.fixture()
        path = "src/core/src/type_system/solver.rs"
        self.write(root, path, (root / path).read_text() + "\nfn unify_rigid_dimensions() {}\n")
        self.assert_failure(root, "rigid-dimension alias path")

    def test_10_compound_evolution_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/solver.rs",
            "| DimensionExpr::Multiply(children)",
            "| DimensionExpr::Hole /* removed multiply */",
        )
        self.assert_failure(root, "evolution omits Multiply")

    def test_11_conversion_authority_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/conversion.rs",
            "plan_implicit_conversion(source, target).is_ok()",
            "source == target",
        )
        self.assert_failure(root, "does not delegate")

    def test_12_integer_f64_funnel_fails(self):
        root = self.fixture()
        path = "src/core/src/type_system/conversion.rs"
        self.replace(
            root,
            path,
            "match real_number(number)? {",
            "number_to_f64(number); match real_number(number)? {",
        )
        self.assert_failure(root, "integer conversion funnels through f64")

    def test_13_runtime_entry_semantic_scheme_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/catalog.rs",
            "pub name: String,",
            "pub name: String,\n    scheme: KindScheme,",
        )
        self.assert_failure(root, "RuntimeFunctionEntry carries")

    def test_14_signature_projection_in_production_fails(self):
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn bad() { scheme_from_signature(); }\n")
        self.assert_failure(root, "signature projection")

    def test_15_named_and_intrinsic_authority_regressions_fail(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/catalog.rs",
            "type_declaration: FunctionTypeDeclaration,",
            "type_declaration: (),",
        )
        self.assert_failure(root, "named source specializers")
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/catalog.rs",
            "type_authority: SourceTypeAuthority::SyntaxDirectedIntrinsic,",
            "type_authority: SourceTypeAuthority::Schemes(type_declaration),",
        )
        self.assert_failure(root, "syntax-directed authority")

    def test_16_runtime_binding_requires_resolved_output(self):
        root = self.fixture()
        path = "src/core/src/function/specialization.rs"
        self.write(
            root,
            path,
            (root / path).read_text()
            + "\nimpl SpecializationContext<'_> { fn bind_runtime_factory(&self) {} }\n",
        )
        self.assert_failure(root, "can bind without an existing ResolvedCall")

    def test_17_output_allocation_probe_fails(self):
        root = self.fixture()
        path = "src/core/src/function/specialization.rs"
        self.write(
            root,
            path,
            (root / path).read_text()
            + "\nimpl SpecializationContext<'_> { fn bind_runtime_factory(&self) {\n"
            + "let _ = self.resolved_output(0);\n"
            + "let _probe = ValueCell::default_for_representation(output_representation, output_dimensions);\n"
            + "let candidates = catalog.runtime_entries();\n} }\n",
        )
        self.assert_failure(root, "output allocation as overload probing")

    def test_18_source_resolution_order_and_output_check_fail(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/function/resolver.rs",
            "let resolved = resolve_declared_call(entry, declaration, invocation)?;",
            "let resolved = ResolvedCall::default();",
        )
        self.assert_failure(root, "resolve semantics before")
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/function/resolver.rs",
            "validate_resolved_output(entry, &resolved, &function)?;",
            "function.output().resolved_type()?;",
        )
        self.assert_failure(root, "close and validate")

    def test_19_named_wildcard_and_wire_leak_fail(self):
        root = self.fixture()
        path = "src/core/src/type_system/scheme.rs"
        self.write(root, path, (root / path).read_text() + "\nfn bad() { KindExpr::Wildcard; }\n")
        self.assert_failure(root, "named semantic declaration contains a wildcard")
        root = self.fixture()
        self.write(root, "src/core/src/program/bytecode/r3.rs", "struct Wire(ConversionPlan);\n")
        self.assert_failure(root, "R3 metadata leaks")

    def test_20_conformance_documentation_ci_and_owner_regressions_fail(self):
        root = self.fixture()
        marker = CHECKER.CONFORMANCE[0]
        for relative in CHECKER.REQUIRED:
            if "/tests/type_system_" in relative:
                path = root / relative
                path.write_text(path.read_text().replace(marker, "missing", 1), encoding="utf-8")
        self.assert_failure(root, "conformance suite is missing")
        root = self.fixture()
        self.replace(root, "docs/design/type-system-v1.md", "Status: R3 semantic solver complete", "Status: R3 in progress")
        self.assert_failure(root, "type-system design is missing")
        root = self.fixture()
        self.replace(root, ".github/workflows/ci.yml", "python3 scripts/check-r3-type-system.py", "true")
        self.assert_failure(root, "does not run the R3")
        root = self.fixture()
        self.replace(root, ".github/ci/owners.toml", "scripts/check-r3-type-system.py", "scripts/missing.py")
        self.assert_failure(root, "owner entry is missing")

    def test_21_closed_predicate_vocabulary_rejects_additions(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/builtin.rs",
            "    Keyable,\n}",
            "    Keyable,\n    LegacyPredicate,\n}",
        )
        self.assert_failure(root, "closed nine-predicate vocabulary")

    def test_22_interval_endpoint_regressions_fail(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/solver.rs",
            "left_max <= right.0",
            "left.0 <= right.0",
        )
        self.assert_failure(root, "left maximum to right minimum")
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/solver.rs",
            "left.0 >= right_max",
            "left.0 >= right.0",
        )
        self.assert_failure(root, "actual minimum to lower maximum")

    def test_23_matrix_product_outer_axis_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/scheme.rs",
            "vec![matrix(kind(2), dim(0), dim(3))]",
            "vec![matrix(kind(2), dim(4), dim(5))]",
        )
        self.assert_failure(root, "dynamic matrix product lost semantic closure")

    def test_24_bounded_variadic_expansion_fails(self):
        root = self.fixture()
        path = "src/core/src/type_system/scheme.rs"
        self.write(root, path, (root / path).read_text() + "\nfn bad() { for _ in 1..=32 {} }\n")
        self.assert_failure(root, "retired 32-argument expansion")

    def test_25_formula_add_storage_probe_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/expressions/formulas.rs",
            "        let invocation = crate::SpecializationInvocation::from_cells(",
            "        lhs.representation();\n        let invocation = crate::SpecializationInvocation::from_cells(",
        )
        self.assert_failure(root, "formula + inspects runtime representation")

    def test_26_resident_conversion_execution_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/resident/conversion.rs",
            "execute_conversion_draft(source, &plan.conversion.step)",
            "Ok(source)",
        )
        self.assert_failure(root, "resident convert/kind execution is missing")

    def test_27_physical_binding_diagnostic_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/function/specialization.rs",
            "execution implementation",
            "runtime factory for representation F64",
        )
        self.assert_failure(root, "diagnostics expose physical binding details")

    def test_28_expression_diagnostic_physical_type_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/expressions/errors.rs",
            "pub(super) found: ResolvedType,",
            "pub(super) found: FunctionValueRepresentation,",
        )
        self.assert_failure(root, "expression diagnostics expose physical binding types")

    def test_29_expression_diagnostic_semantic_name_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/engine/src/expressions/errors.rs",
            "self.found.semantic_name()",
            "format!(\"{:?}\", self.found)",
        )
        self.assert_failure(root, "does not format a semantic type name")

    def test_30_full_ci_resident_conversion_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            ".github/workflows/ci-full.yml",
            "compiled_conversion_executes_after_bytecode_round_trip",
            "missing_resident_conversion_conformance",
        )
        self.assert_failure(root, "does not execute resident conversion conformance")

    def test_31_legacy_predicate_constraint_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/kind_scheme.rs",
            "    Satisfies {\n",
            "    Keyable(KindExpr),\n    Satisfies {\n",
        )
        self.assert_failure(root, "operation-specific Keyable")

    def test_32_public_predicate_evidence_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/resolved.rs",
            "pub(crate) struct KindPredicateEvidence",
            "pub struct KindPredicateEvidence",
        )
        self.assert_failure(root, "KindPredicateEvidence is public")

    def test_33_nonnumeric_promotion_shortcut_regression_fails(self):
        root = self.fixture()
        path = "src/core/src/type_system/conversion.rs"
        self.replace(
            root,
            path,
            ") -> Result<Option<PromotionPlan>, TypeResolutionError> {\n    match",
            ") -> Result<Option<PromotionPlan>, TypeResolutionError> {\n"
            "    if exact_type_equal(left, right) { return Ok(None); }\n    match",
        )
        self.assert_failure(root, "arbitrary exact-equal kinds")

    def test_34_dynamic_dot_dimension_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/type_system/scheme.rs",
            "            KindConstraint::DimensionCompatible(dim(0), dim(2)),\n"
            "            KindConstraint::DimensionCompatible(dim(1), dim(3)),\n"
            "        ],\n"
            "    )\n"
            "}\n\n"
            "pub fn matrix_solve",
            "            KindConstraint::DimensionCompatible(dim(0), dim(2)),\n"
            "            // missing column relation\n"
            "        ],\n"
            "    )\n"
            "}\n\n"
            "pub fn matrix_solve",
        )
        self.assert_failure(root, "dynamic_matrix_dot lost semantic closure")

    def test_35_table_join_constraint_regression_fails(self):
        root = self.fixture()
        self.replace(
            root,
            "src/core/src/kind_scheme.rs",
            "    Satisfies {\n",
            "    TableJoin { left: KindExpr },\n    Satisfies {\n",
        )
        self.assert_failure(root, "operation-specific TableJoin")


if __name__ == "__main__":
    unittest.main()
