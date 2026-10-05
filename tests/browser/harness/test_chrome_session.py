"""Startup failures retain browser diagnostics and release owned resources."""

from __future__ import annotations

import contextlib
import io
import itertools
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tests.browser.harness import chrome


class ChromeSessionStartupTests(unittest.TestCase):
    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.directory = Path(directory.name)
        self.browser = self.directory / "browser"
        self.enterContext(mock.patch.object(chrome, "find_browser", return_value=self.browser))
        self.enterContext(mock.patch.object(chrome, "free_port", return_value=43210))
        self.enterContext(mock.patch.object(chrome.os, "killpg", create=True))
        self.session = chrome.ChromeSession(
            None, self.directory / "profile", self.directory / "chrome.log",
        )

    def process_writing(self, payload: bytes, *, returncode: int | None = None) -> mock.Mock:
        process = mock.Mock(pid=12345, returncode=returncode)
        process.poll.return_value = returncode

        def launch(_args: object, **kwargs: object) -> mock.Mock:
            kwargs["stderr"].write(payload)
            return process

        self.enterContext(mock.patch.object(chrome.subprocess, "Popen", side_effect=launch))
        return process

    def assert_closed(self, process: mock.Mock) -> None:
        process.wait.assert_called_once_with(timeout=10)
        self.assertIsNone(self.session.process)
        self.assertIsNone(self.session._log_handle)

    def test_timeout_reports_endpoint_error_and_bounded_stderr_tail(self) -> None:
        process = self.process_writing(
            b"discarded-prefix\n" + b"x" * 16000 + b"\xffGPU initialization stalled\n"
        )
        diagnostics = io.StringIO()
        with (
            mock.patch.object(
                chrome.time, "monotonic",
                side_effect=itertools.chain((0.0, 0.0), itertools.repeat(31.0)),
            ),
            mock.patch.object(chrome.time, "sleep"),
            mock.patch.object(
                chrome.urllib.request, "urlopen",
                side_effect=OSError("connection refused by test endpoint"),
            ) as request,
            contextlib.redirect_stderr(diagnostics),
            self.assertRaises(chrome.BrowserFailure) as raised,
        ):
            self.session.start()

        failure = str(raised.exception)
        self.assertIn(request.call_args.args[0], failure)
        self.assertIn("30", failure)
        self.assertIn("connection refused by test endpoint", failure)
        output = diagnostics.getvalue()
        self.assertIn(str(self.browser), output)
        self.assertIn(str(self.session.log), output)
        self.assertIn("\ufffdGPU initialization stalled", output)
        self.assertNotIn("discarded-prefix", output)
        self.assertLess(len(output), 17000)
        self.assert_closed(process)

    def test_early_exit_reports_stderr_and_keeps_exit_status(self) -> None:
        process = self.process_writing(b"browser startup rejected\n", returncode=7)
        diagnostics = io.StringIO()
        with (
            contextlib.redirect_stderr(diagnostics),
            self.assertRaises(chrome.BrowserFailure) as raised,
        ):
            self.session.start()

        self.assertIn("browser exited with status 7", str(raised.exception))
        self.assertIn("browser startup rejected", diagnostics.getvalue())
        self.assert_closed(process)

    def test_unavailable_log_does_not_mask_the_original_failure(self) -> None:
        for log_exists in (False, True):
            with self.subTest(log_exists=log_exists):
                if log_exists:
                    self.session.log.mkdir()
                failure = chrome.BrowserFailure("original startup failure")
                diagnostics = io.StringIO()
                with (
                    mock.patch.object(self.session, "_start", side_effect=failure),
                    contextlib.redirect_stderr(diagnostics),
                    self.assertRaises(chrome.BrowserFailure) as raised,
                ):
                    self.session.start()

                self.assertIs(raised.exception, failure)
                self.assertIn(str(self.session.log), diagnostics.getvalue())
                self.assertIsNone(self.session.process)
                self.assertIsNone(self.session._log_handle)


if __name__ == "__main__":
    unittest.main()
