#!/usr/bin/env python3

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "distribution_artifact", ROOT / "scripts" / "build-distribution-artifact.py"
)
BUILDER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDER)

CURRENT_WRAPPER = """export class WasmDocument {
    static fromEncodedWithBundle(encoded, root, sources, resolutions, provenance) {}
    static fromServedEncodedWithBundle(encoded, root, config, sources, resolutions, provenance) {}
    renderedDocumentValue(name) {}
    renderedProgramOutput() {}
    replInvoke(source) {}
    replContinueStep(count, request) {}
}
export class WasmProject {
    static validateStaticSources() {}
    static supportsServedDocumentProvenance() {}
}
"""
WASM_MODULE = b"\0asm\x01\0\0\0\0\x01\0"


class DistributionBrowserRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        root_patch = mock.patch.object(BUILDER, "ROOT", self.root)
        root_patch.start()
        self.addCleanup(root_patch.stop)
        self.package = self.root / "src" / "wasm" / "pkg"

    def write_package(self, wrapper=CURRENT_WRAPPER, wasm=WASM_MODULE):
        self.package.mkdir(parents=True, exist_ok=True)
        (self.package / "mech_wasm.js").write_text(wrapper, encoding="utf-8")
        (self.package / "mech_wasm_bg.wasm").write_bytes(wasm)

    def test_missing_runtime_stops_before_starting_the_native_build(self):
        with mock.patch.object(BUILDER, "run_build") as run:
            with self.assertRaisesRegex(RuntimeError, "runtime is missing or incomplete"):
                BUILDER.build("standard")
        run.assert_not_called()

    def test_partial_package_is_rejected(self):
        self.write_package()
        (self.package / "mech_wasm.js").unlink()
        with self.assertRaisesRegex(RuntimeError, "scripts/build-wasm.py --profile browser"):
            BUILDER.require_browser_package()

    def test_old_document_wrapper_is_rejected(self):
        self.write_package("export class WasmDocument { renderedOutput(interpreter, output) {} }")
        with self.assertRaisesRegex(RuntimeError, "incompatible with the document controller"):
            BUILDER.require_browser_package()

    def test_empty_and_download_error_wasm_are_rejected(self):
        for wasm in (b"\0asm\x01\0\0\0", b"404 Not Found"):
            with self.subTest(wasm=wasm):
                self.write_package(wasm=wasm)
                with self.assertRaisesRegex(RuntimeError, "not a complete WebAssembly module"):
                    BUILDER.require_browser_package()

    def test_matching_runtime_preserves_native_clean_build_timing(self):
        self.write_package()
        target = self.root / "native-target"
        executable = target / "release" / ("mech.exe" if BUILDER.os.name == "nt" else "mech")

        def compile_native(_command):
            executable.parent.mkdir(parents=True, exist_ok=True)
            executable.write_bytes(b"embedded-runtime executable")
            return 2.0

        with mock.patch.dict(BUILDER.os.environ, {"CARGO_TARGET_DIR": str(target)}):
            with mock.patch.object(BUILDER, "run_build", side_effect=compile_native) as run:
                self.assertEqual(BUILDER.build("standard"), (executable, 2.0, 2.0))
        self.assertEqual(run.call_count, 2)
        self.assertEqual(run.call_args_list[0], run.call_args_list[1])


if __name__ == "__main__":
    unittest.main()
