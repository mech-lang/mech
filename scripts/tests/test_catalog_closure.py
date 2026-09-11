"""Validate catalog closure execution and required CI wiring without Rust builds."""

from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
import re
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "catalog_closure_linkage", ROOT / "scripts/check-native-linkage-coverage.py"
)
LINKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LINKAGE)
FULL = (ROOT / ".github/workflows/ci-full.yml").read_text(encoding="utf-8")


def job_block(job):
    match = re.search(
        rf"(?ms)^  {re.escape(job)}:\n(?P<body>.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)",
        FULL,
    )
    if match is None:
        raise AssertionError(f"workflow job {job!r} is missing")
    return match.group("body")


class CatalogClosureCommandTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        root_patch = patch.object(LINKAGE, "ROOT", self.root)
        root_patch.start()
        self.addCleanup(root_patch.stop)

    def invoke(self, *arguments):
        stderr = io.StringIO()
        with patch.object(LINKAGE.sys, "argv", ["check-native-linkage-coverage.py", *arguments]):
            with redirect_stderr(stderr), redirect_stdout(io.StringIO()):
                status = LINKAGE.main()
        return status, stderr.getvalue()

    def report_path(self, profile="standard"):
        return self.root / f"target/native-linkage/catalog-closure-{profile}.json"

    def write_report(self, report, profile="standard"):
        self.report_path(profile).write_text(json.dumps(report), encoding="utf-8")

    def report(self, profile="standard"):
        return {
            "schema": "mech.catalog-closure.v1",
            "profile": f"distribution-{profile}",
            "generated_witness_count": 3,
            "witness_count": 3,
            "compilation_failures": [],
            "witnesses": [{"operation": "logic/not"} for _ in range(3)],
        }

    def test_each_profile_runs_locked_exact_distribution_with_explicit_report_environment(self):
        for profile in ("standard", "full"):
            with self.subTest(profile=profile):
                def complete(command, **kwargs):
                    self.write_report(self.report(profile), profile)
                    return SimpleNamespace(returncode=0)

                inherited = {
                    "MECH_CATALOG_CLOSURE_PROFILE": "wrong-profile",
                    "MECH_CATALOG_CLOSURE_REPORT": "/wrong/report.json",
                }
                with patch.dict(os.environ, inherited):
                    with patch.object(LINKAGE.subprocess, "run", side_effect=complete) as run:
                        status, stderr = self.invoke("catalog-closure", profile)
                    self.assertEqual(os.environ["MECH_CATALOG_CLOSURE_PROFILE"], "wrong-profile")
                self.assertEqual((status, stderr), (0, ""))
                run.assert_called_once()
                self.assertEqual(run.call_args.args[0], [
                    "cargo", "+nightly-2026-03-03", "test", "--locked", "-p", "mech",
                    "--no-default-features", "--features", f"distribution-{profile}",
                    "--test", "catalog_closure", "--", "--nocapture",
                ])
                self.assertEqual(run.call_args.kwargs["cwd"], self.root)
                environment = run.call_args.kwargs["env"]
                self.assertEqual(environment["MECH_CATALOG_CLOSURE_PROFILE"], f"distribution-{profile}")
                self.assertEqual(environment["MECH_CATALOG_CLOSURE_REPORT"], str(self.report_path(profile)))

    def test_only_standard_and_full_profiles_are_accepted_before_running_or_writing(self):
        for profile in ("default", "distribution-standard", "STANDARD", "../full", ""):
            with self.subTest(profile=profile), patch.object(LINKAGE, "run") as run:
                status, stderr = self.invoke("catalog-closure", profile)
                self.assertEqual(status, 1)
                self.assertIn("unknown catalog closure profile", stderr)
                run.assert_not_called()
                self.assertFalse((self.root / "target").exists())

    def test_missing_extra_and_unknown_command_arguments_fail(self):
        for arguments in (("catalog-closure",), ("catalog-closure", "standard", "full"), ("unknown",)):
            with self.subTest(arguments=arguments), patch.object(LINKAGE, "run") as run:
                status, stderr = self.invoke(*arguments)
                self.assertEqual(status, 2)
                self.assertIn("usage:", stderr)
                run.assert_not_called()

    def test_failed_cargo_command_fails_the_gate_and_retains_its_report(self):
        def fail(command, **kwargs):
            self.write_report(self.report())
            return SimpleNamespace(returncode=101)

        with patch.object(LINKAGE.subprocess, "run", side_effect=fail):
            status, stderr = self.invoke("catalog-closure", "standard")
        self.assertEqual(status, 1)
        self.assertIn("command failed (101)", stderr)
        self.assertTrue(self.report_path().is_file())

    def test_success_without_a_fresh_report_cannot_reuse_stale_evidence(self):
        self.report_path().parent.mkdir(parents=True)
        self.write_report(self.report())
        with patch.object(LINKAGE, "run"):
            status, stderr = self.invoke("catalog-closure", "standard")
        self.assertEqual(status, 1)
        self.assertIn("catalog closure validation failed", stderr)
        self.assertFalse(self.report_path().exists())

    def test_report_must_match_schema_profile_and_have_generated_witnesses(self):
        cases = [
            ([], "schema changed"),
            ({**self.report(), "schema": "old"}, "schema changed"),
            ({**self.report(), "profile": "distribution-full"}, "does not match"),
            *[({**self.report(), "witness_count": count}, "no generated witnesses")
              for count in (None, 0, -1, True, "3", 1.5)],
        ]
        for report, message in cases:
            with self.subTest(report=report):
                with patch.object(LINKAGE, "run", side_effect=lambda *a, **kw: self.write_report(report)):
                    status, stderr = self.invoke("catalog-closure", "standard")
                self.assertEqual(status, 1)
                self.assertIn(message, stderr)

    def test_invalid_json_is_reported_as_gate_failure(self):
        with patch.object(LINKAGE, "run", side_effect=lambda *a, **kw: self.report_path().write_text("{")):
            status, stderr = self.invoke("catalog-closure", "standard")
        self.assertEqual(status, 1)
        self.assertIn("catalog closure validation failed", stderr)

    def test_every_generated_witness_must_be_accounted_for_by_a_success(self):
        for count in (None, 0, 2, 4, True, "3", 3.0):
            report = {**self.report(), "generated_witness_count": count}
            with self.subTest(count=count):
                with patch.object(LINKAGE, "run", side_effect=lambda *a, **kw: self.write_report(report)):
                    status, stderr = self.invoke("catalog-closure", "standard")
                self.assertEqual(status, 1)
                self.assertIn("does not account for every generated witness", stderr)

    def test_report_requires_an_empty_compilation_failure_list(self):
        for failures in (None, False, "", {}, [{"operation": "logic/not", "error": "missing factory"}]):
            report = {**self.report(), "compilation_failures": failures}
            with self.subTest(failures=failures):
                with patch.object(LINKAGE, "run", side_effect=lambda *a, **kw: self.write_report(report)):
                    status, stderr = self.invoke("catalog-closure", "standard")
                self.assertEqual(status, 1)
                self.assertIn("compilation failures or lacks a failure list", stderr)

    def test_report_requires_a_witness_list_matching_the_success_count(self):
        for witnesses in (None, False, "abc", {"a": 1, "b": 2, "c": 3}, [], [{}, {}], [{}, {}, {}, {}]):
            report = {**self.report(), "witnesses": witnesses}
            with self.subTest(witnesses=witnesses):
                with patch.object(LINKAGE, "run", side_effect=lambda *a, **kw: self.write_report(report)):
                    status, stderr = self.invoke("catalog-closure", "standard")
                self.assertEqual(status, 1)
                self.assertIn("witness list does not match witness_count", stderr)


class CatalogClosureWorkflowTests(unittest.TestCase):
    def test_ci_runs_both_exact_profiles_and_retains_reports(self):
        block = job_block("catalog-closure")
        matrix = re.search(r"(?ms)matrix:\n\s+profile:\n(?P<rows>(?:\s+- [a-z0-9-]+\n)+)", block)
        self.assertIsNotNone(matrix)
        self.assertEqual(re.findall(r"- ([a-z0-9-]+)", matrix.group("rows")), ["standard", "full"])
        self.assertIn("fail-fast: false", block)
        self.assertIn("ref: ${{ inputs.validation_ref || github.sha }}", block)
        self.assertIn("python3 -B -m unittest scripts/tests/test_catalog_closure.py", block)
        self.assertIn("cargo +nightly-2026-03-03 fetch --locked", block)
        self.assertIn('python3 scripts/check-native-linkage-coverage.py catalog-closure "${{ matrix.profile }}"', block)
        self.assertIn('if test "${{ matrix.profile }}" = standard', block)
        for feature in ("standard_compiler", "full_runtime", "full_source", "full_compiler"):
            self.assertIn(f"--no-default-features --features {feature}", block)
        self.assertEqual(block.count("--test profile_contracts"), 4)
        self.assertIn("uses: actions/upload-artifact@v4", block)
        self.assertIn("if: ${{ always() }}", block)
        self.assertIn("path: target/native-linkage/catalog-closure-${{ matrix.profile }}.json", block)
        self.assertIn("if-no-files-found: error", block)
        self.assertNotIn("continue-on-error", block)

    def test_cargo_aggregate_requires_catalog_closure_success(self):
        block = job_block("cargo")
        self.assertIn("if: ${{ always() }}", block)
        needs = re.search(r"(?ms)^    needs:\n(?P<rows>(?:      - .+\n)+)", block)
        self.assertIsNotNone(needs)
        self.assertIn("      - catalog-closure\n", needs.group("rows"))
        self.assertIn("CATALOG_CLOSURE_RESULT: ${{ needs.catalog-closure.result }}", block)
        self.assertIn('test "$CATALOG_CLOSURE_RESULT" = success', block)
        self.assertNotIn("continue-on-error", block)


if __name__ == "__main__":
    unittest.main()
