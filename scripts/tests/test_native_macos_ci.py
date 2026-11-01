#!/usr/bin/env python3
"""Keep cold owner preparation separate from the macOS fixture execution gate."""

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class NativeMacosCiTests(unittest.TestCase):
    def setUp(self):
        workflow = (ROOT / ".github/workflows/ci-full.yml").read_text()
        self.job = workflow.split("\n  native-macos:\n", 1)[1].split("\n  wasm-bytecode-v1:\n", 1)[0]

    def test_cold_build_is_bounded_and_low_memory(self):
        for setting in ('CARGO_BUILD_JOBS: "1"', 'CARGO_INCREMENTAL: "0"',
                        'CARGO_PROFILE_DEV_DEBUG: "0"', 'CARGO_PROFILE_TEST_DEBUG: "0"'):
            self.assertIn(setting, self.job)
        prepare, execute = self.job.split("- name: Build selected native fixture families", 1)
        self.assertIn('MECH_NATIVE_OWNER_BUILD_TIMEOUT_SECS: "1800"', prepare)
        self.assertIn("--stage macos-owner-prepare --timeout-secs 4200", prepare)
        self.assertIn("--test native_generated_end_to_end -- --ignored --exact prepare_generated_owner_runners", prepare)
        self.assertIn('MECH_NATIVE_OWNER_REQUIRE_PREBUILT: "1"', execute)
        self.assertIn("--stage macos-generated --timeout-secs 5400", execute)

    def test_all_six_cases_and_failure_diagnostics_remain_required(self):
        self.assertIn("MECH_NATIVE_GENERATED_CASE: scalar,fixed-matrix,dynamic-matrix,cli,console,time-once", self.job)
        self.assertIn("--exact every_generated_application_fixture_builds_and_executes --test-threads=1", self.job)
        self.assertIn("if: always()", self.job)
        self.assertIn("target/native-plan-logs", self.job)
        self.assertIn("target/bytecode-v1-fixtures/owner-runner/logs", self.job)
        self.assertNotIn("continue-on-error", self.job)


if __name__ == "__main__":
    unittest.main()
