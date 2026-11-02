"""Focused contracts for explicit browser completion progress."""

from __future__ import annotations

import queue
import unittest

from tests.browser.harness.chrome import BrowserCompletionServer, BrowserFailure


class BrowserCompletionServerTests(unittest.TestCase):
    @staticmethod
    def completion() -> BrowserCompletionServer:
        completion = object.__new__(BrowserCompletionServer)
        completion.messages = queue.Queue()
        completion.last_progress = None
        return completion

    def test_completion_returns_payload_after_progress(self) -> None:
        completion = self.completion()
        completion.messages.put(
            ("ekf-progress", b'{"stage":"document-ready"}')
        )
        completion.messages.put(("ekf-finished", b'{"mechDone":"true"}'))

        self.assertEqual(
            completion.wait_for(
                "ekf-finished",
                timeout=0.1,
                progress=("ekf-progress",),
                max_timeout=0.2,
            ),
            b'{"mechDone":"true"}',
        )
        self.assertIsNotNone(completion.last_progress)

    def test_timeout_reports_the_last_semantic_progress(self) -> None:
        completion = self.completion()
        completion.messages.put(
            (
                "ekf-progress",
                b'{"stage":"first-compute-submit","computeDispatches":"0"}',
            )
        )

        with self.assertRaises(BrowserFailure) as raised:
            completion.wait_for(
                "ekf-finished",
                timeout=0.01,
                progress=("ekf-progress",),
                max_timeout=0.1,
            )

        message = str(raised.exception)
        self.assertIn("idle deadline", message)
        self.assertIn("last progress 'ekf-progress'", message)
        self.assertIn('"stage":"first-compute-submit"', message)

    def test_timeout_reports_when_no_progress_arrived(self) -> None:
        completion = self.completion()
        with self.assertRaises(BrowserFailure) as raised:
            completion.wait_for(
                "ekf-finished",
                timeout=0.01,
                progress=("ekf-progress",),
                max_timeout=0.1,
            )

        self.assertIn("no progress beacon was received", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
