"""Reject dirty or missing imports before producing a pinned block oracle."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "block_reference", ROOT / "benchmarks/mech-deflate/matchfinder/generate_block_reference.py"
)
REFERENCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REFERENCE)


class BlockReferencePinTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.package = self.root / "package"
        self.package.mkdir()
        self.names = ["CompressOptimal", "Matchfinder", "BtMatchfinder",
                      "DeflateTables", "CompressLazy", "HuffmanEncode"]
        for name in self.names:
            (self.package / f"{name}.roc").write_text(f"# pinned {name}\n")
        self.git("init", "--quiet")
        self.git("add", "package")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "--quiet", "-m", "pinned package fixture")
        self.revision = self.git("rev-parse", "HEAD").strip()

    def git(self, *arguments):
        return subprocess.check_output(["git", *arguments], cwd=self.root, text=True)

    def test_all_pinned_sources_are_returned_and_generated_apps_are_not_inputs(self):
        (self.package / "MechFullBlockOracle.roc").write_text("# generated app\n")
        sources = REFERENCE.pinned_package_sources(self.root, self.revision)
        self.assertEqual(set(sources), {f"package/{name}.roc" for name in self.names})

    def test_primary_and_transitive_dependency_changes_are_rejected(self):
        for name in self.names:
            with self.subTest(module=name):
                path = self.package / f"{name}.roc"
                original = path.read_bytes()
                try:
                    path.write_bytes(original + b"# changed\n")
                    with self.assertRaisesRegex(ValueError, name):
                        REFERENCE.pinned_package_sources(self.root, self.revision)
                finally:
                    path.write_bytes(original)

    def test_missing_transitive_dependency_is_rejected(self):
        (self.package / "HuffmanEncode.roc").unlink()
        with self.assertRaisesRegex(ValueError, "HuffmanEncode"):
            REFERENCE.pinned_package_sources(self.root, self.revision)


if __name__ == "__main__":
    unittest.main()
