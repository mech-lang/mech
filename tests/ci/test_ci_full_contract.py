#!/usr/bin/env python3

import json
import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
FULL = (ROOT / ".github/workflows/ci-full.yml").read_text(encoding="utf-8")
F0_CONTROLLED = (ROOT / ".github/workflows/f0-controlled.yml").read_text(
    encoding="utf-8"
)
STATIC = (ROOT / "scripts/check-static-distribution-profiles.sh").read_text(
    encoding="utf-8"
)
SIZE_SCRIPT = ROOT / "scripts/report-distribution-sizes.sh"

FULL_CHECKOUT_REF = "ref: ${{ inputs.validation_ref || github.sha }}"
STATIC_PROFILES = (
    "static",
    "engine",
    "selected-runtime",
    "full-runtime",
    "full-source",
    "full-compiler",
)
SIZE_PROFILES = (
    "selected-bytecode-runtime",
    "full-bytecode-runtime",
    "full-source-runtime",
    "full-compiler-tooling",
    "wasm-browser-project",
)


def job_block(source: str, job: str) -> str:
    match = re.search(
        rf"(?ms)^  {re.escape(job)}:\n(?P<body>.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)",
        source,
    )
    if match is None:
        raise AssertionError(f"workflow job {job!r} is missing")
    return match.group("body")


class FullWorkflowContractTests(unittest.TestCase):
    def test_pr_full_validation_receives_exact_head(self):
        block = job_block(CI, "full-validation")
        self.assertIn(
            "validation_ref: ${{ github.event.pull_request.head.sha }}", block
        )

    def test_reusable_workflow_declares_ref_and_falls_back_for_other_invocations(self):
        self.assertRegex(
            FULL,
            r"(?ms)workflow_call:.*?validation_ref:\n"
            r"\s+description: Exact commit or ref to validate\n"
            r"\s+required: false\n\s+type: string\n\s+default: \"\"",
        )
        self.assertIn(FULL_CHECKOUT_REF, FULL)

    def test_every_repository_checkout_uses_validation_ref(self):
        lines = FULL.splitlines()
        checkouts = [
            index
            for index, line in enumerate(lines)
            if "uses: actions/checkout@" in line
        ]
        self.assertGreater(len(checkouts), 0)
        for index in checkouts:
            with self.subTest(line=index + 1):
                self.assertTrue(
                    any(
                        line.strip() == FULL_CHECKOUT_REF
                        for line in lines[index + 1 : index + 5]
                    )
                )

    def test_f0_protocol_is_checked_without_a_stale_evidence_waiver(self):
        block = job_block(FULL, "architecture-contracts")
        self.assertIn("scripts/check-f0-product-tree.py", block)
        self.assertIn("scripts/f0_contract.py", block)
        self.assertIn("scripts/tests/test_f0_contract.py", block)
        self.assertNotIn("allow-only-c0-gate-b-evidence-stale", block)
        self.assertIn(".evidence != null", block)

    def test_f0_controlled_dispatch_is_explicit_and_exact_head_bound(self):
        self.assertRegex(F0_CONTROLLED, r"(?ms)pull_request:\n\s+types:\n\s+- labeled")
        self.assertIn("github.event.label.name == 'f0-controlled'", F0_CONTROLLED)
        exact_ref = "${{ github.event.pull_request.head.sha || github.sha }}"
        self.assertEqual(F0_CONTROLLED.count(exact_ref), 3)
        self.assertIn("workflow-context.txt", F0_CONTROLLED)
        self.assertIn('test "${F0_VALIDATION_BRANCH}" =', F0_CONTROLLED)

    def test_function_system_job_provisions_ripgrep_for_both_slices(self):
        block = job_block(FULL, "function-system-contracts")
        install = "sudo apt-get install --yes ripgrep"
        self.assertEqual(block.count(install), 1)
        self.assertLess(
            block.index(install),
            block.index(
                'bash scripts/check-function-system-contracts.sh "${{ matrix.slice }}"'
            ),
        )
        self.assertNotRegex(
            block[: block.index(install)],
            r"(?m)^\s+if:\s.*matrix\.slice",
        )

    def test_static_distribution_matrix_uses_only_authoritative_profiles(self):
        block = job_block(FULL, "distribution-contracts")
        matrix = re.search(
            r"(?ms)\n\s+matrix:\n\s+profile:\n(?P<rows>(?:\s+- [a-z0-9-]+\n)+)",
            block,
        )
        self.assertIsNotNone(matrix)
        profiles = tuple(re.findall(r"- ([a-z0-9-]+)", matrix.group("rows")))
        self.assertEqual(profiles, STATIC_PROFILES)
        accepted = re.search(r'(?ms)case "\$mode" in\n\s+(?P<modes>[^;]+) ;;', STATIC)
        self.assertIsNotNone(accepted)
        accepted_profiles = {
            item.strip()
            for item in accepted.group("modes").rstrip(")").split("|")
        }
        self.assertTrue(set(profiles).issubset(accepted_profiles))
        self.assertNotIn("wasm-source", profiles)

    def test_native_linkage_matrix_uses_authoritative_full_surface(self):
        block = job_block(FULL, "native-linkage-surfaces")
        matrix = re.search(
            r"(?ms)\n\s+matrix:\n\s+surface:\n"
            r"(?P<rows>(?:\s+- [a-z0-9-]+\n)+)",
            block,
        )
        self.assertIsNotNone(matrix)
        profiles = tuple(re.findall(r"- ([a-z0-9-]+)", matrix.group("rows")))
        self.assertEqual(profiles[0], "full")
        self.assertNotIn("standard", profiles)

    def test_distribution_size_profiles_are_authoritative_and_deterministic(self):
        completed = subprocess.run(
            [str(SIZE_SCRIPT), "--profiles-json"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        profiles = json.loads(completed.stdout)
        self.assertEqual(tuple(profiles), SIZE_PROFILES)
        self.assertEqual(len(profiles), len(set(profiles)))

        declared = re.search(
            r"(?ms)^supported_profiles='(?P<profiles>[^']+)'$", SIZE_SCRIPT.read_text()
        )
        self.assertIsNotNone(declared)
        self.assertEqual(tuple(declared.group("profiles").splitlines()), SIZE_PROFILES)

    def test_distribution_size_workflow_consumes_one_profile_output(self):
        plan = job_block(FULL, "distribution-size-plan")
        shards = job_block(FULL, "distribution-size-shards")
        combine = job_block(FULL, "distribution-sizes")
        self.assertIn("report-distribution-sizes.sh --profiles-json", plan)
        self.assertIn(
            "profile: ${{ fromJSON(needs.distribution-size-plan.outputs.profiles) }}",
            shards,
        )
        self.assertIn(
            "PROFILES_JSON: ${{ needs.distribution-size-plan.outputs.profiles }}",
            combine,
        )
        self.assertNotIn("continue-on-error", shards)
        self.assertNotIn("continue-on-error", combine)
        self.assertIn('test "$missing" -eq 0', combine)
        for stale in (
            "standard-bytecode-runtime",
            "standard-source-runtime",
            "standard-compiler-tooling",
        ):
            self.assertNotIn(stale, FULL)

    def test_unknown_distribution_size_profile_still_fails(self):
        completed = subprocess.run(
            [str(SIZE_SCRIPT), "/tmp/unused-distribution-size.tsv", "unknown-profile"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("unknown distribution profile", completed.stderr)


if __name__ == "__main__":
    unittest.main()
