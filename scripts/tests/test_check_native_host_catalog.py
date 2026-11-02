from __future__ import annotations

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "check-native-host-catalog.py"
SPEC = importlib.util.spec_from_file_location("check_native_host_catalog", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class NativeHostCatalogCheckerTests(unittest.TestCase):
    def check_source(self, source: str) -> tuple[int, str]:
        # Exercise registration checks without invoking Cargo in mutation fixtures.
        packages = []
        for host in ("terminal", "console", "time", "timer", "scene", "robot-arm"):
            manifest = tomllib.loads(
                (CHECKER.ROOT / "hosts" / host / "Cargo.toml").read_text(encoding="utf-8")
            )
            packages.append({
                "name": manifest["package"]["name"],
                "dependencies": [{
                    "name": "mech-runtime", "kind": None,
                    "features": manifest["dependencies"]["mech-runtime"]["features"],
                }],
            })
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary) / "standard.rs"
            fixture.write_text(source, encoding="utf-8")
            output = io.StringIO()
            with patch.object(CHECKER, "STANDARD", fixture), patch.object(
                CHECKER, "capture", return_value=json.dumps({"packages": packages})
            ), contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                result = CHECKER.main()
            return result, output.getvalue()

    def test_explicit_argument_factory_passes(self):
        result, message = self.check_source(CHECKER.STANDARD.read_text(encoding="utf-8"))
        self.assertEqual(result, 0, message)

    def test_process_global_factory_cannot_replace_explicit_arguments(self):
        source = CHECKER.STANDARD.read_text(encoding="utf-8")
        original = 'factory_path: "mech_terminal::CliHostFactory::with_arguments"'
        self.assertEqual(source.count(original), 1)
        # A matching constructor name elsewhere must not satisfy the CLI registration.
        source = source.replace(original, 'factory_path: "mech_terminal::CliHostFactory::new"', 1)
        source += '\nconst UNRELATED: &str = "mech_terminal::CliHostFactory::with_arguments";\n'
        result, message = self.check_source(source)
        self.assertEqual(result, 1, message)
        self.assertIn("cli launch factory must receive explicit program arguments", message)

    def test_explicit_argument_input_cannot_be_dropped(self):
        source = CHECKER.STANDARD.read_text(encoding="utf-8")
        original = "NativeHostFactoryInput::ProgramArguments,"
        self.assertIn(original, source)
        source = source.replace(original, "NativeHostFactoryInput::None,", 1)
        result, message = self.check_source(source)
        self.assertEqual(result, 1, message)
        self.assertIn("cli factory input must be ProgramArguments", message)

    def test_comments_cannot_supply_the_actual_factory_field(self):
        original = 'factory_path: "mech_terminal::CliHostFactory::with_arguments"'
        for comment in ("// " + original, "/* nested /* marker */ " + original + " */"):
            with self.subTest(comment=comment):
                source = CHECKER.STANDARD.read_text(encoding="utf-8")
                source = source.replace(
                    original,
                    comment + '\n                factory_path: "mech_terminal::CliHostFactory::new"',
                    1,
                )
                result, message = self.check_source(source)
                self.assertEqual(result, 1, message)
                self.assertIn("cli launch factory must receive explicit program arguments", message)

    def test_literal_text_cannot_supply_the_actual_factory_field(self):
        source = CHECKER.STANDARD.read_text(encoding="utf-8")
        original = 'factory_path: "mech_terminal::CliHostFactory::with_arguments"'
        source = source.replace(
            original,
            'factory_path: { let _note = r#"factory_path: "mech_terminal::CliHostFactory::with_arguments""#; "mech_terminal::CliHostFactory::new" }',
            1,
        )
        result, message = self.check_source(source)
        self.assertEqual(result, 1, message)
        self.assertIn("cli launch factory must receive explicit program arguments", message)


if __name__ == "__main__":
    unittest.main()
