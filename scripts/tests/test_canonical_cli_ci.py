#!/usr/bin/env python3
"""Synthetic receipt checks only; never launch Cargo, Mech or a benchmark."""

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check_canonical_cli_ci.py"
SPEC = importlib.util.spec_from_file_location("canonical_cli_ci", SCRIPT)
CI = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CI)


class CanonicalCliEvidenceTests(unittest.TestCase):
    def write_json(self, path, value):
        path.write_text(json.dumps(value), encoding="utf-8")

    def test_selection_requires_exact_names_count_and_no_duplicates(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "selection.log"
            log.write_text("\n".join(name + ": test" for name in CI.PUBLIC_NAMES))
            CI.selection(log, CI.PUBLIC_NAMES)
            for names in ([], ["other"], list(CI.PUBLIC_NAMES) + [next(iter(CI.PUBLIC_NAMES))]):
                log.write_text("\n".join(name + ": test" for name in names))
                with self.assertRaises(ValueError):
                    CI.selection(log, CI.PUBLIC_NAMES)

    def test_execution_requires_single_nonzero_exact_pass_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "execution.log"
            good = "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
            log.write_text(good)
            CI.passed_count(log, 3)
            for value in ("", good + good, good.replace("3 passed", "0 passed"),
                          good.replace("0 ignored", "3 ignored")):
                log.write_text(value)
                with self.assertRaises(ValueError):
                    CI.passed_count(log, 3)

    def test_stage_requires_successful_exit_and_completed_owned_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            summary = {"stage": "case", "reason": "exit", "command_exit_status": 0,
                       "owned_group_cleanup": "complete"}
            self.write_json(root / "case.summary.json", summary)
            CI.completed_summary(root, "case")
            for key, value in (("reason", "timeout"), ("command_exit_status", 1),
                               ("owned_group_cleanup", "unknown")):
                self.write_json(root / "case.summary.json", {**summary, key: value})
                with self.assertRaises(ValueError):
                    CI.completed_summary(root, "case")

    def pin_fixture(self, root):
        for label in ("unit", "public", "mech"):
            (root / label).write_bytes(label.encode())
        for role, label in (("lib", "unit"), ("public", "public")):
            (root / f"canonical-cli-{role}-executable").write_text(str(root / label) + "\n")
        self.write_json(root / "canonical-cli-public-compile.log", {
            "reason": "compiler-artifact", "target": {"name": "mech", "kind": ["bin"]},
            "profile": {"test": False}, "executable": str(root / "mech"),
        })

    def test_actual_reported_executables_are_pinned_and_rechecked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.pin_fixture(root)
            CI.pin_executables(root)
            CI.verify_pins(root)
            (root / "mech").write_bytes(b"changed")
            with self.assertRaises(ValueError):
                CI.verify_pins(root)
            with self.assertRaises(ValueError):
                CI.pin_executables(root)

    def evidence_fixture(self, root):
        for prefix, children in CI.CASE_CHILDREN.items():
            case = root / (prefix + "fixture")
            case.mkdir()
            self.write_json(case / "qualification-result.json", {
                "schema": "mech.public-native-cli-qualification.v1", "passed": True,
            })
            for label, success in children.items():
                for stream in ("stdout", "stderr"):
                    (case / f"{label}.{stream}").write_bytes(f"{label}:{stream}".encode())
                self.write_json(case / f"{label}.result.json", {
                    "state": "completed", "qualification_only": True,
                    "exit_code": 0 if success else 1, "success": success,
                    **{f"{stream}_sha256": CI.digest(case / f"{label}.{stream}")
                       for stream in ("stdout", "stderr")},
                })
        unused = root / "unused-observation-fixture"
        (unused / "denied-launch.json").write_text("{}")
        (unused / "accepted-launch.json").write_text("{}")

    def test_all_three_cases_and_twelve_exact_child_outcomes_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.evidence_fixture(root)
            CI.case_evidence(root)
            case = root / "zero-resources-fixture"
            (case / "closed-run.stdout").write_bytes(b"drift")
            with self.assertRaises(ValueError):
                CI.case_evidence(root)
            (case / "qualification-result.json").unlink()
            with self.assertRaises((ValueError, FileNotFoundError)):
                CI.case_evidence(root)

    def test_launch_policies_and_failed_child_outcomes_cannot_disappear(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.evidence_fixture(root)
            unused = root / "unused-observation-fixture"
            (unused / "denied-launch.json").unlink()
            with self.assertRaises(ValueError):
                CI.case_evidence(root)


if __name__ == "__main__":
    unittest.main()
