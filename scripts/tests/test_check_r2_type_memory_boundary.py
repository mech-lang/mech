from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-r2-type-memory-boundary.py"
SPEC = importlib.util.spec_from_file_location("check_r2_type_memory_boundary", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class TypeMemoryBoundaryCheckerTests(unittest.TestCase):
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

    def test_repository_boundary_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_runtime_storage_is_private(self):
        root = self.fixture()
        self.replace(root, "src/core/src/lib.rs", "pub(crate) mod runtime_storage;", "pub mod runtime_storage;")
        self.assert_failure(root, "runtime_storage must not be public")

    def test_memory_contract_cannot_acquire_physical_authority(self):
        for declaration in ("type Backing = ValueCell;", "struct Physical { byte_offset: usize }", "type Platform = GpuBuffer;", "unsafe fn escape() {}"):
            with self.subTest(declaration=declaration):
                root = self.fixture()
                self.write(root, "src/core/src/memory_contract/probe.rs", declaration)
                self.assertTrue(CHECKER.failures(root))

    def test_type_memory_contract_cannot_become_wire_data(self):
        root = self.fixture()
        self.write(root, "src/engine/src/artifact/probe.rs", "struct Artifact { contract: TypeMemoryContract }")
        self.assert_failure(root, "leaks into wire-format code")

    def test_serialization_outside_known_wire_roots_is_rejected(self):
        root = self.fixture()
        self.write(root, "src/core/src/probe.rs", "fn encode(contract: TypeMemoryContract) {}")
        self.assert_failure(root, "serialization implementation outside the wire roots")

    def test_alias_validation_uses_writable_storage_authority(self):
        root = self.fixture()
        path = root / "src/core/src/function/argument.rs"
        path.write_text(path.read_text().replace("same_writable_storage", "same_logical_cell"))
        self.assert_failure(root, "operation alias checker")


if __name__ == "__main__":
    unittest.main()
