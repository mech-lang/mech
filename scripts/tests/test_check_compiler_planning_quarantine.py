from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check-compiler-planning-quarantine.py"
SPEC = importlib.util.spec_from_file_location("compiler_planning_quarantine", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class CompilerPlanningQuarantineTests(unittest.TestCase):
    def fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        files = {
            "src/core/src/lib.rs": "pub mod source_diagnostic;\npub mod encoded_payload;\n",
            "src/engine/src/lib.rs": "pub mod memory_runtime;\n",
            "src/engine/src/program/mod.rs": '#[cfg(feature = "semantic-compiler")]\nmod compiler_planning;\n',
            "src/engine/src/program/compiler_planning.rs": "pub struct CompilerPlanningConfig;\n",
            "src/engine/src/artifact/encoding.rs": 'const DOMAIN: &[u8] = b"mech-program-v1\\0";\n',
            "src/runtime/src/runtime/program/compiler.rs": "use mech_core::LegacyValue;\n",
            "src/runtime/src/runtime/program/external/value_adapter_tests.rs": "use mech_core::LegacyValue;\n",
            "src/runtime/src/runtime/program/value.rs": "use mech_core::LegacyValue;\n",
        }
        for relative, source in files.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source, encoding="utf-8")
        return root

    def test_exact_private_compiler_boundary_passes(self):
        self.assertEqual(CHECKER.run(self.fixture()), [])

    def test_public_interpreter_and_old_instance_fail(self):
        root = self.fixture()
        (root / "src/engine/src/lib.rs").write_text(
            '#[cfg(feature = "semantic-compiler")]\npub mod interpreter;\n',
            encoding="utf-8",
        )
        instance = root / "src/engine/src/program/instance.rs"
        instance.write_text("pub struct MechProgram;\n", encoding="utf-8")
        failures = CHECKER.run(root)
        self.assertTrue(any("retired interpreter module remains reachable" in row for row in failures))
        self.assertTrue(any("obsolete mutable program instance" in row for row in failures))
        self.assertTrue(any("removed MechProgram surface" in row for row in failures))

    def test_shipping_executor_call_fails(self):
        root = self.fixture()
        product = root / "src/build/src/product.rs"
        product.parent.mkdir(parents=True, exist_ok=True)
        product.write_text("fn ship(p: &mut X) { p.run_bytecode(bytes); }\n", encoding="utf-8")
        failures = CHECKER.run(root)
        self.assertTrue(any("shipping run_bytecode reachability" in row for row in failures))

    def test_public_interpreter_reference_alias_fails(self):
        root = self.fixture()
        interpreter = root / "src/engine/src/interpreter/mod.rs"
        interpreter.parent.mkdir(parents=True)
        interpreter.write_text(
            "pub type InterpreterRef = Ref<Box<Interpreter>>;\n",
            encoding="utf-8",
        )
        failures = CHECKER.run(root)
        self.assertTrue(any("removed InterpreterRef surface" in row for row in failures))
        self.assertTrue(any("retired AST workspace remains" in row for row in failures))

    def test_disabled_private_interpreter_restoration_fails(self):
        root = self.fixture()
        (root / "src/engine/src/lib.rs").write_text(
            '#[cfg(any())]\nmod interpreter;\n', encoding="utf-8"
        )
        failures = CHECKER.run(root)
        self.assertTrue(any("retired interpreter module remains reachable" in row for row in failures))

    def test_disabled_ast_visitor_export_restoration_fails(self):
        for module in ("expressions", "literals", "structures"):
            with self.subTest(module=module):
                root = self.fixture()
                (root / "src/engine/src/lib.rs").write_text(
                    f'#[cfg(any())]\npub use {module}::*;\n', encoding="utf-8"
                )
                self.assertTrue(any(f"retired {module} module remains reachable" in row for row in CHECKER.run(root)))

    def test_disabled_core_source_tree_export_restoration_fails(self):
        for source in (
            '#[cfg(any())]\npub mod nodes;\n',
            '#[cfg(any())]\npub use self::nodes::*;\n',
        ):
            with self.subTest(source=source):
                root = self.fixture()
                (root / "src/core/src/lib.rs").write_text(source, encoding="utf-8")
                self.assertTrue(any("retired source-tree module/export remains reachable" in row for row in CHECKER.run(root)))

    def test_disabled_planning_workspace_restoration_fails(self):
        root = self.fixture()
        planning = root / "src/engine/src/program/compiler_planning.rs"
        planning.write_text(
            '#[cfg(any())]\nstruct CompilerPlanningProgram;\n', encoding="utf-8"
        )
        failures = CHECKER.run(root)
        self.assertTrue(any("removed CompilerPlanningProgram surface" in row for row in failures))

    def test_core_ast_formatting_helper_restoration_fails(self):
        for source in (
            '#[cfg(any())]\npub struct IndexedString { pub data: Vec<char> }\n',
            '#[cfg(any())]\nimpl IndexedString { fn new() {} }\n',
        ):
            with self.subTest(source=source):
                root = self.fixture()
                (root / "src/core/src/lib.rs").write_text(source, encoding="utf-8")
                self.assertTrue(any("retired IndexedString AST formatting helper remains" in row for row in CHECKER.run(root)))

    def test_unrelated_indexed_string_name_is_not_retired(self):
        root = self.fixture()
        unrelated = root / "src/build/src/text_index.rs"
        unrelated.parent.mkdir(parents=True)
        unrelated.write_text("struct IndexedString;\n", encoding="utf-8")
        self.assertEqual(CHECKER.run(root), [])

    def test_source_namespace_identity_does_not_admit_an_executor_type(self):
        root = self.fixture()
        index = root / "src/runtime/src/resolver/index.rs"
        index.parent.mkdir(parents=True)
        index.write_text(
            "enum SourceScope { Program, Interpreter(SourceInterpreterId) }\n"
            "fn named_scope() { SourceScope::Interpreter(id); }\n",
            encoding="utf-8",
        )
        self.assertEqual(CHECKER.run(root), [])
        with index.open("a", encoding="utf-8") as source:
            source.write("\n#[cfg(any())] struct Interpreter;\n")
        self.assertTrue(any("removed Interpreter surface" in row for row in CHECKER.run(root)))

    def test_removed_ast_path_restoration_fails_even_without_exports(self):
        for relative in CHECKER.REMOVED_WORKSPACE_PATHS:
            with self.subTest(relative=relative):
                root = self.fixture()
                path = root / relative
                if not path.suffix:
                    path = path / "mod.rs"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("// inert but physically retired workspace\n", encoding="utf-8")
                self.assertTrue(any("retired AST workspace remains" in row for row in CHECKER.run(root)))

    def test_legacy_value_exception_is_exact(self):
        root = self.fixture()
        sibling = root / "src/runtime/src/runtime/program/external/other.rs"
        sibling.write_text("use mech_core::LegacyValue;\n", encoding="utf-8")
        failures = CHECKER.run(root)
        self.assertTrue(any("outside an exact approved adapter" in row for row in failures))

    def test_compatibility_domain_literal_is_allowed(self):
        root = self.fixture()
        failures = CHECKER.run(root)
        self.assertFalse(any("mech-program-v1" in row for row in failures))

    def test_deleted_runtime_error_import_fails_before_compilation(self):
        root = self.fixture()
        compiler = root / "src/runtime/src/runtime/program/compiler.rs"
        compiler.write_text(
            "use mech_core::LegacyValue;\n"
            "#[cfg(any())]\n"
            "use mech_engine::expressions::ReactiveComprehensionStructureUnsupported;\n",
            encoding="utf-8",
        )
        # The previous physical/export and executor checks missed this consumer.
        self.assertEqual(
            CHECKER.check_module_boundary(root)
            + CHECKER.check_removed_surface(root)
            + CHECKER.check_shipping_reachability(root),
            [],
        )
        self.assertEqual(
            CHECKER.run(root),
            ["src/runtime/src/runtime/program/compiler.rs:3: retired mech_engine::expressions namespace"],
        )

    def test_disabled_qualified_retired_imports_fail_for_every_owner(self):
        for owner, modules in CHECKER.RETIRED_NAMESPACES.items():
            for module in modules:
                for path in (
                    f"{owner}::{module}::OldSurface",
                    f"::{owner} /* root */ :: /* member */ {module} :: OldSurface",
                    f"r#{owner}::r#{module}::OldSurface",
                ):
                    with self.subTest(path=path):
                        root = self.fixture()
                        product = root / "src/runtime/src/runtime/program/compiler.rs"
                        product.write_text(f"#[cfg(any())]\nuse {path} as Old;\n", encoding="utf-8")
                        self.assertTrue(any(
                            f"retired {owner}::{module} namespace" in row
                            for row in CHECKER.run(root)
                        ))

    def test_grouped_retired_imports_fail_without_banning_nested_local_names(self):
        for owner, module in (
            ("mech_engine", "expressions"),
            ("mech_core", "nodes"),
        ):
            for path in (
                f"{owner}::{{allowed::{{{module}}}, {module}::OldSurface}}",
                f"{owner}::{{allowed, self::{{{module}::OldSurface}}}}",
                f"{{{owner}::{{allowed, self::{module}::OldSurface}}, project::Other}}",
            ):
                with self.subTest(path=path):
                    root = self.fixture()
                    product = root / "src/runtime/src/runtime/program/compiler.rs"
                    product.write_text(f"#[cfg(any())]\nuse {path};\n", encoding="utf-8")
                    self.assertEqual(CHECKER.check_retired_namespaces(root), [
                        f"src/runtime/src/runtime/program/compiler.rs:2: retired {owner}::{module} namespace"
                    ])

    def test_qualified_namespace_restoration_is_checked_in_all_maintained_rust_roots(self):
        for source_root in ("src", "machines", "hosts", "tests"):
            with self.subTest(source_root=source_root):
                root = self.fixture()
                relative = f"{source_root}/restoration_probe.rs"
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    "#[cfg(any())]\nfn unused(_: mech_engine::literals::OldSurface) {}\n",
                    encoding="utf-8",
                )
                self.assertEqual(CHECKER.check_retired_namespaces(root), [
                    f"{relative}:2: retired mech_engine::literals namespace"
                ])

    def test_retired_paths_in_comments_and_quoted_negative_fixtures_are_not_imports(self):
        root = self.fixture()
        negative = root / "tests/quoted_scanner_negative.rs"
        negative.parent.mkdir(parents=True)
        negative.write_text(
            '// mech_engine::interpreter::OldSurface\n'
            '/* mech_engine::expressions::OldSurface\n'
            '   /* mech_engine::literals::OldSurface */\n'
            '   mech_core::nodes::OldSurface */\n'
            'const A: &str = "use mech_engine::structures::OldSurface;";\n'
            'const B: &str = r###"use mech_engine::{expressions::OldSurface};"###;\n'
            'const C: &[u8] = b"use mech_core::nodes::OldSurface;";\n'
            'const D: &[u8] = br##"mech_engine::interpreter::OldSurface"##;\n'
            'const E: &CStr = c"mech_engine::literals::OldSurface";\n'
            'const F: &CStr = cr#"mech_engine::structures::OldSurface"#;\n'
            'const G: &str = "escaped \\\" mech_core::nodes::OldSurface";\n'
            "const CH: char = '\"'; const BYTE: u8 = b'\"';\n",
            encoding="utf-8",
        )
        self.assertEqual(CHECKER.run(root), [])

    def test_comments_and_literals_do_not_hide_a_real_disabled_import(self):
        root = self.fixture()
        compiler = root / "src/runtime/src/runtime/program/compiler.rs"
        compiler.write_text(
            'const QUOTE: char = \'"\';\r\n'
            '/* outer /* inner */ tail */\r\n'
            'const FIXTURE: &str = r#"mech_core::nodes::OldSurface"#;\r\n'
            '#[cfg(any())]\r\n'
            'use mech_engine::structures::OldSurface;\r\n',
            encoding="utf-8",
        )
        self.assertEqual(CHECKER.check_retired_namespaces(root), [
            "src/runtime/src/runtime/program/compiler.rs:5: retired mech_engine::structures namespace"
        ])

    def test_canonical_intrinsic_and_unrelated_namespace_names_remain_allowed(self):
        root = self.fixture()
        product = root / "src/runtime/src/runtime/program/compiler.rs"
        product.write_text(
            "use mech_engine::{source_semantics::frontend, intrinsics::{aggregate, kind_conversion}};\n"
            "use mech_core::{source_diagnostic, encoded_payload, execution::ComputePlacement};\n"
            "use project::{expressions, interpreter, literals, structures, nodes};\n"
            "mod expressions {} mod interpreter {} mod literals {} mod structures {} mod nodes {}\n",
            encoding="utf-8",
        )
        self.assertEqual(CHECKER.run(root), [])

    def test_external_owner_names_inside_local_paths_are_not_external_namespaces(self):
        root = self.fixture()
        product = root / "src/runtime/src/runtime/program/compiler.rs"
        product.write_text(
            "use project::mech_engine::expressions::Local;\n"
            "use crate /* local */ :: mech_core :: nodes :: Local;\n"
            "use project::{mech_engine::structures::Local, local::{mech_core::nodes::Local}};\n"
            "use {project::{mech_engine::literals::Local}, mech_engine::intrinsics::aggregate};\n",
            encoding="utf-8",
        )
        self.assertEqual(CHECKER.run(root), [])


if __name__ == "__main__":
    unittest.main()
