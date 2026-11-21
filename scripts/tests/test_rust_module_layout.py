import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "check-rust-module-layout.py"
SPEC = importlib.util.spec_from_file_location("rust_module_layout", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class RustModuleLayoutTests(unittest.TestCase):
    def check(self, files):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            for name in files:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("// fixture\n")
            output = io.StringIO()
            with patch.object(CHECKER, "REPOSITORY_ROOT", root), patch.object(
                CHECKER, "workspace_source_roots", return_value={root}
            ), contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                return CHECKER.main(), output.getvalue()

    def test_local_folder_modules_use_mod_rs(self):
        status, _ = self.check(["src/child/mod.rs", "src/child/tests.rs"])
        self.assertEqual(status, 0)

    def test_local_file_and_folder_collision_is_rejected(self):
        status, output = self.check(["src/child.rs", "src/child/tests.rs"])
        self.assertEqual(status, 1)
        self.assertIn("src/child.rs: file/directory module collision", output)

    def test_vendor_keeps_upstream_layout_under_an_enclosing_source_root(self):
        status, _ = self.check([
            "vendor/upstream/src/child.rs", "vendor/upstream/src/child/tests.rs"
        ])
        self.assertEqual(status, 0)

    def test_similar_named_local_directory_is_still_checked(self):
        status, _ = self.check(["vendor_support/child.rs", "vendor_support/child/tests.rs"])
        self.assertEqual(status, 1)
