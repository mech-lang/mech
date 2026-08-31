from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r1-compatibility-closure.py"
SPEC = importlib.util.spec_from_file_location("check_r1_compatibility_closure", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class R1CompatibilityClosureTests(unittest.TestCase):
    def fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in (
            "src/engine/src/artifact/compiler.rs",
            "src/runtime/src/resource.rs",
            "src/engine/src/intrinsics/assign/catalog.rs",
            "machines/set/src/catalog.rs",
            "machines/math/src/lib.rs",
            "machines/logic/src/not.rs",
        ):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("pub struct Canonical;\n", encoding="utf-8")
        return root

    def test_canonical_tree_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_retired_symbols_fail(self):
        for symbol in (
            "LegacyOpaqueOperationContract",
            "RuntimeResidentResourceWriteRequest",
            "prepare_resident_write",
            "RuntimeOperationIdentity",
            "compile_legacy_bytecode_program_artifact",
            "compile_source_frozen_v1",
            "compile_frozen_v1_program_product",
        ):
            with self.subTest(symbol=symbol):
                root = self.fixture()
                path = root / "src/engine/src/artifact/compiler.rs"
                path.write_text(f"fn probe(_: {symbol}) {{}}\n", encoding="utf-8")
                self.assertTrue(any(symbol in item for item in CHECKER.failures(root)))

    def test_fallback_variants_fail_in_artifact_scope(self):
        root = self.fixture()
        path = root / "src/engine/src/artifact/compiler.rs"
        path.write_text("enum CompilerOp { LegacyCall, FrozenV1 }\n", encoding="utf-8")
        failures = CHECKER.failures(root)
        self.assertTrue(any("LegacyCall" in item for item in failures))
        self.assertTrue(any("FrozenV1" in item for item in failures))

    def test_inferred_metadata_helpers_fail(self):
        root = self.fixture()
        path = root / "machines/set/src/catalog.rs"
        path.write_text("macro_rules! set_runtime_contract {}\n", encoding="utf-8")
        self.assertTrue(any("set_runtime_contract" in item for item in CHECKER.failures(root)))

    def test_implementation_namespace_fails(self):
        root = self.fixture()
        path = root / "src/engine/src/resident/numeric.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            'let operation = vec!["runtime", "AddF64"];\n',
            encoding="utf-8",
        )
        self.assertTrue(
            any(
                "implementation-identity operation namespace" in item
                for item in CHECKER.failures(root)
            )
        )

    def test_math_placeholders_fail(self):
        root = self.fixture()
        path = root / "machines/math/src/lib.rs"
        path.write_text("fn compile() { todo!() }\n", encoding="utf-8")
        self.assertTrue(any("placeholder" in item for item in CHECKER.failures(root)))

    def test_retired_product_claims_fail(self):
        root = self.fixture()
        path = root / "machines/math/Cargo.toml"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('hypot = ["functions"]\n', encoding="utf-8")
        self.assertTrue(any("advertised" in item for item in CHECKER.failures(root)))

    def test_retired_paths_fail(self):
        root = self.fixture()
        path = root / CHECKER.RETIRED_PATHS[0]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("retired\n", encoding="utf-8")
        self.assertTrue(any("path exists" in item for item in CHECKER.failures(root)))


if __name__ == "__main__":
    unittest.main()
