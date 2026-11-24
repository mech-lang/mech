import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "served_program_smoke", ROOT / "scripts/smoke-served-program-results-browser.py"
)
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)


class ServedProgramSmokeTests(unittest.TestCase):
    def test_stopped_server_cannot_pass_as_an_invalid_document(self):
        with mock.patch.object(SMOKE, "browser_result") as result:
            with self.assertRaisesRegex(SMOKE.BrowserFailure, "server terminated"):
                SMOKE.checked_browser_result(None, None, "http://example/", ROOT, [], 1, invalid=True)
        result.assert_not_called()

    def test_error_response_requires_a_visible_browser_diagnostic(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            with mock.patch.object(SMOKE, "browser_result", side_effect=SMOKE.BrowserFailure("invisible")) as result:
                with self.assertRaisesRegex(SMOKE.BrowserFailure, "invisible"):
                    SMOKE.checked_browser_result(None, (400, "syntax/missing-delimiter"),
                                                 "http://example/", artifacts, [], 1, invalid=True)
            result.assert_called_once_with(None, "http://example/", artifacts, [], 1, invalid=True)

    def test_embedded_mode_passes_no_local_package_override(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.object(SMOKE.subprocess, "Popen") as popen:
                popen.return_value.poll.return_value = 0
                with SMOKE.serve(Path("/mech"), None, root / "main.mec", root):
                    pass
            command = popen.call_args.args[0]
            self.assertNotIn("--wasm", command)
            self.assertNotIn("--allow-read", command)


if __name__ == "__main__":
    unittest.main()
