"""Startup failures retain browser diagnostics and release owned resources."""

from __future__ import annotations

import contextlib
from collections.abc import Callable
import io
import itertools
import json
from pathlib import Path
import socket
import tempfile
import threading
import time
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
        self.session = chrome.ChromeSession(
            None, self.directory / "profile", self.directory / "chrome.log",
            startup_timeout=30,
        )
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

    def test_default_startup_budget_accepts_readiness_after_thirty_seconds(self) -> None:
        process = self.process_writing(b"")
        version = mock.Mock()
        version.get.return_value = "ws://127.0.0.1:43210/browser"
        devtools = mock.Mock()
        devtools.call.side_effect = [
            {"targetId": "target"}, {"sessionId": "session"}, {}, {},
        ]
        try:
            with (
                mock.patch.object(chrome.time, "monotonic", side_effect=(0.0, 0.0, 31.0)),
                mock.patch.object(chrome.time, "sleep"),
                mock.patch.object(
                    chrome.urllib.request, "urlopen",
                    side_effect=[OSError("not ready yet"), mock.MagicMock()],
                ),
                mock.patch.object(chrome.json, "load", return_value=version),
                mock.patch.object(chrome, "DevTools", return_value=devtools) as connect,
            ):
                self.assertIs(self.session.start(), self.session)

            connect.assert_called_once_with(process, version.get.return_value)
            self.assertIs(self.session.process, process)
            self.assertIs(self.session.devtools, devtools)
            self.assertEqual(self.session.session_id, "session")
            process.wait.assert_not_called()
        finally:
            self.session.close()
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


class ChromeSessionNavigationTests(unittest.TestCase):
    def setUp(self) -> None:
        with mock.patch.object(chrome, "find_browser", return_value=Path("fixture-browser")):
            self.session = chrome.ChromeSession(None, "fixture-profile", "fixture.log")
        client_socket, peer_socket = socket.socketpair()
        client_socket.settimeout(0.01)
        peer_socket.settimeout(1)
        self.client_socket = client_socket
        self.peer_socket = peer_socket
        self.addCleanup(self.close_socket, client_socket)
        self.addCleanup(self.close_socket, peer_socket)
        client = chrome.WebSocket.__new__(chrome.WebSocket)
        client.socket = client_socket
        self.peer = chrome.WebSocket.__new__(chrome.WebSocket)
        self.peer.socket = peer_socket
        process = mock.Mock()
        process.poll.return_value = None
        with mock.patch.object(chrome, "WebSocket", return_value=client):
            self.session.devtools = chrome.DevTools(process, "ws://fixture")
        self.session.session_id = "fixture-session"

    @staticmethod
    def close_socket(connection: socket.socket) -> None:
        try:
            connection.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        connection.close()

    def respond(
        self, send_response: Callable[[dict[str, object]], None],
    ) -> tuple[threading.Thread, list[Exception]]:
        errors = []

        def run() -> None:
            try:
                request = self.peer.receive()
                self.assertEqual(request["method"], "Page.navigate")
                self.assertEqual(request["sessionId"], "fixture-session")
                send_response(request)
            except Exception as error:
                errors.append(error)

        worker = threading.Thread(target=run, daemon=True)
        worker.start()
        self.addCleanup(worker.join, 1)
        return worker, errors

    def test_navigation_accepts_a_reply_after_the_receive_idle_timeout(self) -> None:
        def reply(request: dict[str, object]) -> None:
            time.sleep(0.06)
            self.peer.send(json.dumps({"id": request["id"], "result": {"frameId": "ready"}}))

        worker, errors = self.respond(reply)
        self.session.navigate("http://fixture", timeout=0.5)
        worker.join(1)
        self.assertFalse(worker.is_alive())
        self.assertEqual(errors, [])

    def test_navigation_preserves_partial_header_and_payload_across_receive_timeouts(self) -> None:
        def reply(request: dict[str, object]) -> None:
            payload = json.dumps({"id": request["id"], "result": {"frameId": "ready"}}).encode()
            self.assertLess(len(payload), 126)
            frame = b"\x81" + bytes([len(payload)]) + payload
            for chunk in (frame[:1], frame[1:2], frame[2:5], frame[5:]):
                self.peer_socket.sendall(chunk)
                time.sleep(0.04)

        worker, errors = self.respond(reply)
        self.session.navigate("http://fixture", timeout=0.5)
        worker.join(1)
        self.assertFalse(worker.is_alive())
        self.assertEqual(errors, [])

    def test_navigation_deadline_remains_bounded_without_a_reply(self) -> None:
        started = time.monotonic()
        with self.assertRaisesRegex(chrome.BrowserFailure, "Page.navigate timed out"):
            self.session.navigate("http://fixture", timeout=0.03)
        self.assertLess(time.monotonic() - started, 1)


if __name__ == "__main__":
    unittest.main()
