#!/usr/bin/env python3

import json
import re
import subprocess
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
FULL = (ROOT / ".github/workflows/ci-full.yml").read_text(encoding="utf-8")
NATIVE = (ROOT / ".github/workflows/ci-native-plan.yml").read_text(encoding="utf-8")
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


def normal_static_contracts() -> str:
    return "\n".join(
        job_block(CI, job)
        for job in (
            "static-architecture",
            "static-mutations",
            "static-distribution",
            "static-contracts",
        )
    )


def full_architecture_contracts() -> str:
    return "\n".join(
        job_block(FULL, job)
        for job in ("architecture-contracts", "architecture-mutations")
    )


class FullWorkflowContractTests(unittest.TestCase):
    def test_native_plan_starts_early_once_on_the_exact_head(self):
        early = job_block(CI, "early-native-plan")
        delegated = job_block(FULL, "native-plan")
        caller = job_block(CI, "full-validation")
        self.assertIn("needs: impact", early)
        self.assertNotIn("needs:\n", early)
        self.assertIn("if: needs.impact.outputs.full_validation_required == 'true'", early)
        self.assertIn("validation_ref: ${{ github.event.pull_request.head.sha }}", early)
        self.assertIn("native_plan_in_caller: true", caller)
        for block in (early, delegated):
            self.assertIn("uses: ./.github/workflows/ci-native-plan.yml", block)
            self.assertNotIn("steps:", block)
            self.assertNotIn("continue-on-error", block)
        self.assertIn("if: ${{ !inputs.native_plan_in_caller }}", delegated)
        self.assertIn("validation_ref: ${{ inputs.validation_ref || github.sha }}", delegated)
        call, dispatch = FULL.split("  workflow_dispatch:", 1)
        self.assertRegex(call, r"(?s)native_plan_in_caller:.*?type: boolean\n        default: false")
        self.assertNotIn("native_plan_in_caller:", dispatch.split("\nconcurrency:", 1)[0])
        self.assertIn("- early-native-plan", job_block(CI, "pr-gate"))
        self.assertIn("- native-plan", job_block(FULL, "cargo"))

        native = job_block(NATIVE, "native-plan")
        self.assertIn("ref: ${{ inputs.validation_ref }}", native)
        self.assertNotIn("continue-on-error", native)
        pair = "cargo +nightly-2026-03-03 test -p mech-build --all-features --test registry_generated_project"
        pruning = "cargo +nightly-2026-03-03 test -p mech-build --all-features --test native_host_pruning"
        self.assertLess(native.index(pair), native.index(pruning))
        self.assertEqual(re.findall(r"--skip ([a-z_]+)", native), [
            "registry_project_is_exact_unpatched_and_buildable_with_a_test_only_patch",
            "live_registry_project_runs_once_handles_ctrlc_and_cleans_up_after_failure",
        ])
        for script in ("check-native-host-catalog.py", "check-generated-project-determinism.py", "check-native-application-graphs.py"):
            self.assertIn(f"python3 scripts/{script}", native)

    def test_native_plan_handoff_gates_fail_closed(self):
        def accepts(block, **overrides):
            script = textwrap.dedent(block.split("        run: |\n", 1)[1])
            environment = dict.fromkeys(re.findall(r"^          ([A-Z0-9_]+):", block, re.M), "success")
            environment.update(overrides)
            return subprocess.run(
                ["/bin/bash", "-e", "-c", script], env=environment,
                capture_output=True, text=True,
            ).returncode == 0

        pr = job_block(CI, "pr-gate")
        cargo = job_block(FULL, "cargo")
        self.assertTrue(accepts(pr, DOCS_ONLY="false", FULL_REQUIRED="true"))
        for result in ("failure", "cancelled", "skipped", ""):
            with self.subTest(result=result):
                self.assertFalse(accepts(pr, DOCS_ONLY="false", FULL_REQUIRED="true", NATIVE_PLAN_RESULT=result))
                self.assertFalse(accepts(cargo, NATIVE_PLAN_IN_CALLER="false", NATIVE_PLAN_RESULT=result))
        self.assertTrue(accepts(pr, DOCS_ONLY="false", FULL_REQUIRED="false", FULL_RESULT="skipped", NATIVE_PLAN_RESULT="skipped"))
        self.assertTrue(accepts(cargo, NATIVE_PLAN_IN_CALLER="false"))
        self.assertTrue(accepts(cargo, NATIVE_PLAN_IN_CALLER="true", NATIVE_PLAN_RESULT="skipped"))
        self.assertFalse(accepts(cargo, NATIVE_PLAN_IN_CALLER="true", NATIVE_PLAN_RESULT="failure"))

    def test_architecture_mutations_are_bounded_parallel_exact_head_shards(self):
        normal = job_block(CI, "static-mutations")
        full = job_block(FULL, "architecture-mutations")
        for block, checkout in (
            (normal, "ref: ${{ github.event.pull_request.head.sha }}"),
            (full, FULL_CHECKOUT_REF),
        ):
            with self.subTest(checkout=checkout):
                self.assertIn("shard: [0, 1, 2, 3]", block)
                self.assertIn("timeout-minutes: 8", block)
                self.assertIn("--shard-count 4", block)
                self.assertIn("--shard-index ${{ matrix.shard }}", block)
                self.assertIn("scripts/tests/test_check_r6_memory_runtime.py", block)
                self.assertIn(checkout, block)
                self.assertNotIn("continue-on-error", block)

        aggregate = job_block(CI, "static-contracts")
        for dependency in (
            "static-architecture",
            "static-mutations",
            "static-distribution",
        ):
            self.assertIn(f"- {dependency}", aggregate)
        self.assertIn('test "$ARCHITECTURE_RESULT" = success', aggregate)
        self.assertIn('test "$MUTATIONS_RESULT" = success', aggregate)
        self.assertIn('test "$DISTRIBUTION_RESULT" = success', aggregate)

    def test_ci_tool_installs_ignore_unrelated_apt_sources(self):
        for workflow_name, workflow in (("CI", CI), ("Full CI", FULL)):
            installs = workflow.count("sudo apt-get install --yes ripgrep")
            with self.subTest(workflow=workflow_name):
                self.assertGreater(installs, 0)
                self.assertEqual(
                    workflow.count(
                        "Dir::Etc::sourcelist=/etc/apt/sources.list.d/ubuntu.sources"
                    ),
                    installs,
                )
                self.assertEqual(
                    workflow.count("Dir::Etc::sourceparts=-"), installs
                )

    def test_browser_suites_run_in_parallel_behind_one_required_gate(self):
        standard = job_block(CI, "browser-standard-canary")
        nbody = job_block(CI, "browser-nbody-reference")
        compute = job_block(CI, "browser-compute-canary")
        aggregate = job_block(CI, "browser-canary")

        self.assertIn("Build standard WASM and the standard server", standard)
        self.assertIn("Verify resident rendering without browser errors", standard)
        self.assertNotIn("Verify N-body physics against independent references", standard)
        self.assertIn("Verify N-body physics against independent references", nbody)
        self.assertIn("Build mixed compute WASM and refresh the server", compute)
        self.assertIn("--profile browser-compute-canary", compute)
        self.assertIn("Verify report-only particle WebGPU execution", compute)
        self.assertIn("Verify scalar and WebGPU EKF rendering", compute)
        for dependency in (
            "browser-standard-canary",
            "browser-nbody-reference",
            "browser-compute-canary",
        ):
            self.assertIn(f"- {dependency}", aggregate)
        self.assertIn('test "$STANDARD_RESULT" = success', aggregate)
        self.assertIn('test "$NBODY_RESULT" = success', aggregate)
        self.assertIn('test "$COMPUTE_RESULT" = success', aggregate)

        self.assertIn("smoke-served-resident-nbody-browser.sh", standard)
        self.assertNotIn("smoke-gpu-particles-browser.py", standard)
        self.assertIn("smoke-gpu-particles-browser.py", compute)
        self.assertIn("smoke-served-resident-ekf-browser.sh", compute)
        self.assertNotIn("smoke-served-resident-nbody-browser.sh", compute)

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

    def test_value_system_absence_and_permanent_contracts_are_unwaived(self):
        static = normal_static_contracts()
        architecture = full_architecture_contracts()
        permanent = "python3 scripts/check-value-system-contract.py"
        absence = "python3 scripts/check-no-retired-value-system.py"

        for block in (static, architecture):
            self.assertIn(permanent, block)
            self.assertIn(absence, block)
            self.assertNotIn("generate-value-system-inventory.py", block)
            self.assertNotIn("continue-on-error", block)

    def test_r2_type_memory_boundary_is_unwaived(self):
        r1 = "python3 scripts/check-r1-compatibility-closure.py"
        r2 = "python3 scripts/check-r2-type-memory-boundary.py"
        unit = "scripts/tests/test_check_r2_type_memory_boundary.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(r1, block)
            self.assertIn(r2, block)
            self.assertLess(block.index(r1), block.index(r2))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)
        full = job_block(FULL, "architecture-contracts")
        for token in (
            "cargo +nightly-2026-03-03 test",
            "--all-features",
            "--test type_memory_contract",
            "--test storage_capability",
            "--test operation_memory_requirement",
            "--test type_memory_boundary",
        ):
            self.assertIn(token, full)

    def test_r3_type_system_is_unwaived_and_full_conformance_is_owned(self):
        r2 = "python3 scripts/check-r2-type-memory-boundary.py"
        r3 = "python3 scripts/check-r3-type-system.py"
        unit = "scripts/tests/test_check_r3_type_system.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(r2, block)
            self.assertIn(r3, block)
            self.assertLess(block.index(r2), block.index(r3))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)
        full = job_block(FULL, "architecture-contracts")
        for target in (
            "type_system_builtin",
            "type_system_solver",
            "type_system_conversion",
            "type_system_catalog",
            "type_system_source",
        ):
            self.assertIn(target, full)

    def test_r4_type_cutover_is_unwaived_and_full_conformance_is_owned(self):
        r3 = "python3 scripts/check-r3-type-system.py"
        r4 = "python3 scripts/check-r4-type-cutover.py"
        unit = "scripts/tests/test_check_r4_type_cutover.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(r3, block)
            self.assertIn(r4, block)
            self.assertLess(block.index(r3), block.index(r4))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)
        full = job_block(FULL, "architecture-contracts")
        self.assertIn("Execute the complete R4 conformance boundary", full)
        self.assertGreaterEqual(full.count("--test r4_type_cutover"), 3)

    def test_r6_memory_runtime_and_miri_are_required_exact_head_gates(self):
        r5 = "python3 scripts/check-r5-memory-planner.py"
        r6 = "python3 scripts/check-r6-memory-runtime.py"
        unit = "scripts/tests/test_check_r6_memory_runtime.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(r5, block)
            self.assertIn(r6, block)
            self.assertLess(block.index(r5), block.index(r6))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)

        runtime = job_block(FULL, "managed-memory-runtime")
        fixed = job_block(FULL, "managed-memory-fixed-profiles")
        miri = job_block(FULL, "managed-memory-miri")
        cargo = job_block(FULL, "cargo")
        self.assertIn(FULL_CHECKOUT_REF, runtime)
        self.assertIn(FULL_CHECKOUT_REF, fixed)
        self.assertIn(FULL_CHECKOUT_REF, miri)
        self.assertIn("--test r6_memory_runtime", runtime)
        self.assertIn("--test r6_memory_safety", runtime)
        safety_features = "--features functions,u8,u64,f64,string,matrixd"
        self.assertGreaterEqual(runtime.count(safety_features), 2)
        self.assertIn("--release -p mech-core", runtime)
        self.assertIn("-C debug-assertions=no", runtime)
        self.assertIn("--features standard_compiler", runtime)
        self.assertIn("--features full_compiler", runtime)
        for feature in (
            "matrix1",
            "matrix2",
            "matrix3",
            "matrix4",
            "matrix2x3",
            "matrix3x2",
            "row_vector2",
            "row_vector3",
            "row_vector4",
            "vector2",
            "vector3",
            "vector4",
        ):
            self.assertIn(f"- {feature}", fixed)
        self.assertNotIn("for fixed in", fixed)
        self.assertIn('--features "full_compiler,${{ matrix.feature }}"', fixed)
        self.assertIn("--test r6_managed_functions", fixed)
        self.assertIn("miri test --locked", miri)
        self.assertIn(safety_features, miri)
        self.assertIn("- managed-memory-runtime", cargo)
        self.assertIn("- managed-memory-fixed-profiles", cargo)
        self.assertIn("- managed-memory-miri", cargo)
        self.assertIn('test "$MANAGED_MEMORY_RESULT" = success', cargo)
        self.assertIn('test "$MANAGED_FIXED_PROFILES_RESULT" = success', cargo)
        self.assertIn('test "$MANAGED_MIRI_RESULT" = success', cargo)

    def test_r6_retains_resident_live_admission_unit_tests(self):
        runtime = job_block(FULL, "managed-memory-runtime")
        commands = [
            " ".join(line.split())
            for line in runtime.replace("\\\n", " ").splitlines()
        ]
        self.assertIn(
            "cargo +nightly-2026-03-03 test --locked -p mech-engine "
            "--no-default-features --features full_compiler,resident-artifact "
            "--lib resident::general::live::",
            commands,
        )
        self.assertNotIn("continue-on-error", runtime)

    def test_r1_artifact_closure_prefetches_before_offline_native_build(self):
        block = job_block(FULL, "artifact-closure")
        fetch = "cargo fetch --locked"
        closure = "python3 scripts/check-r1-artifact-closure.py ${{ matrix.representative }}"
        self.assertIn(fetch, block)
        self.assertLess(block.index(fetch), block.index(closure))

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
        self.assertNotIn("extended-math", profiles)
        self.assertEqual(
            tuple(profile for profile in profiles if profile.startswith("extended-math-")),
            (
                "extended-math-shard-unsigned-small",
                "extended-math-shard-unsigned-wide",
                "extended-math-shard-signed-small",
                "extended-math-shard-signed-wide",
                "extended-math-shard-float",
                "extended-math-shard-complex",
                "extended-math-shard-rational",
            ),
        )

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

    def test_distribution_size_measurement_is_not_a_full_ci_gate(self):
        self.assertNotIn("distribution-size-plan:", FULL)
        self.assertNotIn("distribution-size-shards:", FULL)
        self.assertNotIn("distribution-sizes:", FULL)
        self.assertNotIn("report-distribution-sizes.sh", FULL)

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
