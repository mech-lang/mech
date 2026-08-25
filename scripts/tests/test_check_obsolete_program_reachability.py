import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check-obsolete-program-reachability.py"
SPEC = importlib.util.spec_from_file_location("obsolete_program_reachability", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class ObsoleteProgramReachabilityTests(unittest.TestCase):
    def scan(self, files):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path, content in files.items():
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(content, encoding="utf-8")
            return CHECKER.findings(root)

    def test_exact_encoding_hash_domain_literal_is_allowed(self):
        findings = self.scan(
            {
                "src/engine/src/artifact/encoding.rs":
                    'hash.update(b"mech-program-v1\\0");\n'
            }
        )
        self.assertEqual(findings, [])

    def test_mech_program_cargo_dependency_is_rejected(self):
        findings = self.scan(
            {"Cargo.toml": '[dependencies]\nmech-program = { path = "src/program" }\n'}
        )
        self.assertEqual(len(findings), 1)

    def test_mech_program_rust_import_is_rejected(self):
        findings = self.scan({"src/lib.rs": "use mech_program::Program;\n"})
        self.assertEqual(len(findings), 1)

    def test_mech_program_rust_call_is_rejected(self):
        findings = self.scan({"src/lib.rs": "mech_program::execute(program);\n"})
        self.assertEqual(len(findings), 1)

    def test_mech_program_css_selector_literal_is_allowed(self):
        findings = self.scan(
            {"tests/style_contract.rs": 'assert!(css.contains(".mech-program {"));\n'}
        )
        self.assertEqual(findings, [])

    def test_obsolete_feature_or_package_path_is_rejected(self):
        for content in (
            '[features]\nmech-program = []\n',
            'mech-program = { path = "src/program" }\n',
        ):
            with self.subTest(content=content):
                self.assertEqual(len(self.scan({"Cargo.toml": content})), 1)

    def test_domain_literal_is_allowed_only_at_exact_owner(self):
        findings = self.scan(
            {"src/engine/src/other.rs": 'hash.update(b"mech-program-v1\\0");\n'}
        )
        self.assertEqual(len(findings), 1)


if __name__ == "__main__":
    unittest.main()
