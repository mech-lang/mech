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
            "src/engine/src/lib.rs": (
                '#[cfg(feature = "semantic-compiler")]\nmod interpreter;\n'
                '#[cfg(feature = "semantic-compiler")]\n'
                'pub(crate) use interpreter::Interpreter;\n'
            ),
            "src/engine/src/interpreter/mod.rs": (
                "pub(crate) type InterpreterRef = Ref<Box<Interpreter>>;\n"
            ),
            "src/engine/src/program/mod.rs": '#[cfg(feature = "semantic-compiler")]\nmod compiler_planning;\n',
            "src/engine/src/program/compiler_planning.rs": "pub struct CompilerPlanningProgram;\n",
            "src/engine/src/artifact/encoding.rs": 'const DOMAIN: &[u8] = b"mech-program-v1\\0";\n',
            "src/runtime/src/runtime/program/compiler.rs": "use mech_core::LegacyValue;\n",
            "src/runtime/src/runtime/program/external/value_adapter.rs": "use mech_core::LegacyValue;\n",
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
        self.assertTrue(any("interpreter module is public" in row for row in failures))
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
        (root / "src/engine/src/interpreter/mod.rs").write_text(
            "pub type InterpreterRef = Ref<Box<Interpreter>>;\n",
            encoding="utf-8",
        )
        failures = CHECKER.run(root)
        self.assertTrue(any("InterpreterRef remains public" in row for row in failures))

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


if __name__ == "__main__":
    unittest.main()
