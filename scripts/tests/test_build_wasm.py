import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "build-wasm.py"
SPEC = importlib.util.spec_from_file_location("build_wasm", SCRIPT)
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


class WorkshopBuildTests(unittest.TestCase):
    def setUp(self):
        self.run = self.enterContext(patch.object(BUILD, "run"))
        self.remove = self.enterContext(patch.object(BUILD.shutil, "rmtree"))
        self.which = self.enterContext(patch.object(
            BUILD.shutil, "which", return_value="/tools/wasm-bindgen",
        ))
        self.metadata = self.enterContext(patch.object(
            BUILD.subprocess, "run",
            return_value=SimpleNamespace(stdout=json.dumps({
                "target_directory": "/custom-target",
            })),
        ))
        self.files = self.enterContext(patch.object(Path, "is_file", return_value=True))
        self.glue = self.enterContext(patch.object(
            Path, "read_text", return_value="\n".join(BUILD.PROFILES["browser-workshop"][1]),
        ))

    def invoke(self, *arguments):
        with patch("sys.argv", [str(SCRIPT), "--profile", "browser-workshop", *arguments]):
            BUILD.main()

    def test_workshop_keeps_expected_host_and_document_exports(self):
        self.invoke()
        self.assertIn("browser_workshop", self.run.call_args.args)
        self.assertIn("--release", self.run.call_args.args)
        self.metadata.assert_not_called()
        self.which.assert_not_called()

    def test_strip_regenerates_paired_bindings_from_actual_cargo_target(self):
        self.invoke("--strip-debug-names", "--wasm-bindgen", "/tools/wasm-bindgen")
        self.run.assert_called_with(
            "/tools/wasm-bindgen",
            "/custom-target/wasm32-unknown-unknown/release/mech_wasm.wasm",
            "--target", "web", "--out-dir", str(BUILD.PACKAGE),
            "--remove-name-section",
        )
        self.assertIn("--locked", self.metadata.call_args.args[0])
        self.assertIn("--offline", self.metadata.call_args.args[0])

    def test_missing_bindgen_fails_before_removing_existing_package(self):
        self.which.return_value = None
        with self.assertRaisesRegex(SystemExit, "requires the matching wasm-bindgen"):
            self.invoke("--strip-debug-names")
        self.remove.assert_not_called()
        self.run.assert_not_called()

    def test_missing_raw_artifact_does_not_run_bindgen(self):
        self.files.return_value = False
        with self.assertRaisesRegex(SystemExit, "compiled WASM artifact is missing"):
            self.invoke("--strip-debug-names")
        self.assertEqual(self.run.call_count, 2)

    def test_missing_scene_export_rejects_incomplete_workshop_package(self):
        self.glue.return_value = "export class WasmDocument"
        with self.assertRaisesRegex(SystemExit, "export class WasmSceneProgram"):
            self.invoke()


if __name__ == "__main__":
    unittest.main()
