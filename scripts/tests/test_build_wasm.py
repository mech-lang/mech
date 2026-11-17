import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "build_wasm", Path(__file__).resolve().parents[1] / "build-wasm.py"
)
build_wasm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build_wasm)


class BuildWasmTests(unittest.TestCase):
    def run_profile(self, *options, rewrite_lock=False, reject_metadata=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "src" / "wasm" / "pkg"
            package.mkdir(parents=True)
            old_package = package / "old"
            old_package.touch()
            lock = root / "Cargo.lock"
            lock.write_text("tracked lock\n")
            commands = []

            def run(command, **kwargs):
                self.assertEqual(kwargs["cwd"], root)
                self.assertTrue(kwargs["check"])
                commands.append(list(command))
                if command[0] == "cargo" and reject_metadata:
                    raise subprocess.CalledProcessError(101, command)
                if command[0] == "wasm-pack":
                    package.mkdir()
                    exports = build_wasm.PROFILES["browser-compute-canary"][1]
                    (package / "mech_wasm.js").write_text("\n".join(exports))
                    (package / "mech_wasm_bg.wasm").touch()
                    if rewrite_lock:
                        lock.write_text("rewritten lock\n")

            arguments = ["build-wasm.py", "--profile", "browser-compute-canary", *options]
            with (
                patch.object(build_wasm, "ROOT", root),
                patch.object(build_wasm, "PACKAGE", package),
                patch.object(sys, "argv", arguments),
                patch.object(build_wasm.subprocess, "run", side_effect=run),
            ):
                if reject_metadata:
                    with self.assertRaises(subprocess.CalledProcessError):
                        build_wasm.main()
                    self.assertTrue(old_package.exists())
                elif rewrite_lock and "--locked" in options:
                    with self.assertRaisesRegex(SystemExit, "changed Cargo.lock"):
                        build_wasm.main()
                else:
                    build_wasm.main()
            return commands

    def test_locked_mode_checks_metadata_before_passing_locked_to_cargo_build(self):
        metadata, rustup, build = self.run_profile("--locked")
        self.assertEqual(
            metadata,
            ["cargo", "metadata", "--locked", "--format-version", "1",
             "--manifest-path", "src/wasm/Cargo.toml"],
        )
        self.assertEqual(rustup, ["rustup", "target", "add", "wasm32-unknown-unknown"])
        self.assertEqual(build[:3], ["wasm-pack", "build", "src/wasm"])
        self.assertEqual(
            build[-4:], ["--no-default-features", "--features", "browser_compute_canary", "--locked"]
        )

    def test_locked_mode_rejects_wasm_pack_metadata_rewrites(self):
        self.run_profile("--locked", rewrite_lock=True)

    def test_stale_metadata_leaves_the_old_package_in_place(self):
        self.assertEqual(len(self.run_profile("--locked", reject_metadata=True)), 1)

    def test_default_mode_keeps_the_local_build_workflow(self):
        rustup, build = self.run_profile(rewrite_lock=True)
        self.assertEqual(rustup[0], "rustup")
        self.assertNotIn("--locked", build)


if __name__ == "__main__":
    unittest.main()
