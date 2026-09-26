"""Mutation checks for release package metadata, not historical ABI evidence."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "check-package-versions.py"
SPEC = importlib.util.spec_from_file_location("check_package_versions", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PackageVersions(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write("Cargo.toml", '[package]\nname="mech"\nversion="0.4.0-beta"\n'
                   '[dependencies]\nmech-core={path="src/core",version="0.4.0-beta"}\n')
        self.write("src/core/Cargo.toml", '[package]\nname="mech-core"\nversion="0.4.0-beta"\n')
        self.write("Cargo.lock", 'version=4\n[[package]]\nname="mech-core"\nversion="0.4.0-beta"\n')

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_matching_beta_metadata(self):
        self.assertEqual(MODULE.check(self.root), [])

    def test_stale_package(self):
        self.write("src/core/Cargo.toml", '[package]\nname="mech-core"\nversion="0.3.5"\n')
        self.assertIn("package version", "\n".join(MODULE.check(self.root)))

    def test_stale_requirement(self):
        self.write("Cargo.toml", '[package]\nname="mech"\nversion="0.4.0-beta"\n'
                   '[target."cfg(unix)".build-dependencies]\ncore_alias={package="mech-core",version="0.3.5"}\n')
        self.assertIn("core_alias requirement", "\n".join(MODULE.check(self.root)))

    def test_stale_lock(self):
        self.write("Cargo.lock", 'version=4\n[[package]]\nname="mech-core"\nversion="0.3.5"\n')
        self.assertIn("lock version", "\n".join(MODULE.check(self.root)))

    def test_independently_versioned_helper(self):
        self.write("benchmarks/iros-2026/blog/render/Cargo.toml", '[package]\nname="mech-iros-blog-render"\nversion="0.1.0"\n'
                   '[dependencies]\nmech-core={path="../../../../src/core"}\n')
        self.assertEqual(MODULE.check(self.root), [])

    def test_historical_evidence_is_outside_release_metadata(self):
        self.write("benchmarks/iros-2026/results/archive.json", '{"mech_version":"0.3.5"}')
        self.write("src/core/src/bytecode.rs", 'const ABI_VERSION: (u8,u8,u8) = (0,3,5);')
        self.assertEqual(MODULE.check(self.root), [])


if __name__ == "__main__":
    unittest.main()
