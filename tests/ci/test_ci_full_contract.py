#!/usr/bin/env python3

import ast
import html
import json
import os
import re
import shlex
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
FULL = (ROOT / ".github/workflows/ci-full.yml").read_text(encoding="utf-8")
NATIVE = (ROOT / ".github/workflows/ci-native-plan.yml").read_text(encoding="utf-8")
NATIVE_APPLICATIONS = (ROOT / ".github/actions/native-applications/action.yml").read_text(encoding="utf-8")
NATIVE_STABILIZATION = (ROOT / ".github/workflows/ci-native-stabilization.yml").read_text(encoding="utf-8")
NATIVE_DETERMINISM = (
    ROOT / "scripts/check-generated-project-determinism.py"
).read_text(encoding="utf-8")
NATIVE_APPLICATION_GRAPHS = (
    ROOT / "scripts/check-native-application-graphs.py"
).read_text(encoding="utf-8")
NATIVE_GENERATED_END_TO_END = (
    ROOT / "src/build/tests/native_generated_end_to_end.rs"
).read_text(encoding="utf-8")
NATIVE_PLANNING = (ROOT / "src/build/tests/planning.rs").read_text(encoding="utf-8")
NBODY_BROWSER = (ROOT / "scripts/smoke-served-resident-nbody-browser.sh").read_text(
    encoding="utf-8"
)
EKF_BROWSER = (ROOT / "scripts/smoke-served-resident-ekf-browser.sh").read_text(
    encoding="utf-8"
)
BROWSER_HARNESS = (ROOT / "tests/browser/harness/chrome.py").read_text(
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
INTEGRATION_CHECKOUT_JOBS = {
    "impact",
    "standard-linux",
    "changed-owner-tests",
    "standard-windows",
    "browser-standard-canary",
    "browser-nbody-reference",
}
EXACT_HEAD_CHECKOUT_JOBS = {
    "static-architecture",
    "static-mutations",
    "static-distribution",
    "browser-compute-build",
    "browser-compute-smoke",
    "browser-ekf-cpu",
    "browser-ekf-wgpu",
    "browser-ekf-parity",
}


def job_block(source: str, job: str) -> str:
    match = re.search(
        rf"(?ms)^  {re.escape(job)}:\n(?P<body>.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)",
        source,
    )
    if match is None:
        raise AssertionError(f"workflow job {job!r} is missing")
    return match.group("body")


def job_steps(source: str, job: str) -> list[str]:
    body = job_block(source, job).split("    steps:\n", 1)[1]
    starts = [match.start() for match in re.finditer(r"(?m)^      - ", body)]
    return [
        body[start : starts[index + 1] if index + 1 < len(starts) else len(body)]
        for index, start in enumerate(starts)
    ]


def checkout_jobs(source: str) -> set[str]:
    return {
        match.group(1)
        for match in re.finditer(r"(?m)^  ([a-z0-9][a-z0-9-]*):\n", source)
        if "uses: actions/checkout@" in job_block(source, match.group(1))
    }


def workflow_checkout_identity(source: str) -> str:
    match = re.search(
        r"(?ms)^env:\n  RECORD_CHECKOUT_IDENTITY: \|\n(?P<body>.*?)(?=^jobs:\n)",
        source,
    )
    if match is None:
        raise AssertionError("workflow-owned checkout identity verifier is missing")
    return textwrap.dedent(match.group("body")).strip()


CHECKOUT_IDENTITY = workflow_checkout_identity(CI)


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


def literal_assignment(source: str, name: str):
    module = ast.parse(source)
    for statement in module.body:
        if not isinstance(statement, ast.Assign):
            continue
        if any(isinstance(target, ast.Name) and target.id == name for target in statement.targets):
            return ast.literal_eval(statement.value)
    raise AssertionError(f"assignment {name!r} is missing")


class FullWorkflowContractTests(unittest.TestCase):
    def test_native_mixed_cli_product_coverage_is_selected_and_nonempty(self):
        block = job_block(FULL, "cargo-runtime")
        step = next(
            step for step in job_steps(FULL, "cargo-runtime")
            if "Verify native mixed compute through source and inline CLI commands" in step
        )
        self.assertIn("--features distribution-full,compute_backends_native", step)
        self.assertEqual(step.count("--test mech_cli_host native_mixed_compute_"), 2)
        self.assertIn("-- --list", step)
        self.assertIn('test "$CLI_MIXED_TEST_COUNT" -eq 3', step)
        self.assertIn("-- --nocapture --test-threads=1", step)
        self.assertIn('CARGO_BUILD_JOBS: "2"', step)
        self.assertIn('CARGO_PROFILE_DEV_DEBUG: "0"', step)
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', step)
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("if:", step)
        self.assertIn("--features distribution-standard", block)

    def test_static_compute_product_uses_matching_build_and_is_a_required_browser_step(self):
        smoke = job_block(CI, "browser-compute-smoke")
        step = next(
            step for step in job_steps(CI, "browser-compute-smoke")
            if "Verify emitted static CPU and WebGPU bundles under a non-root prefix" in step
        )
        self.assertLess(smoke.index("Restore the shared compute browser build"), smoke.index(step))
        self.assertLess(smoke.index("Install Node for static compute package admission"), smoke.index(step))
        self.assertIn("scripts/smoke-canonical-compute-browser.py", step)
        self.assertIn("--static-bundle --mech-bin target/debug/mech", step)
        self.assertIn("--wasm-pkg src/wasm/pkg --software-adapter", step)
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("if:", step)
        self.assertNotIn("--disable-gpu", step)
        self.assertIn("browser-compute-smoke", job_block(CI, "browser-compute-canary"))

    def test_r06_acceptance_and_shape_facts_are_explicitly_selected(self):
        block = job_block(FULL, "cargo-language")
        step = next(
            step
            for step in job_steps(FULL, "cargo-language")
            if "Verify canonical document execution and rendered outputs" in step
        )
        self.assertIn("--features full_source,resident-routing-source", step)
        self.assertIn("--test s8_recovery_logical_activation_facts", step)
        self.assertIn("--features full_source,resident-artifact,r64,c64", step)
        self.assertIn("--lib resident::general::shape_fact_tests::", step)
        self.assertIn(step, block)

    def test_landing_source_fixture_is_fetched_before_offline_execution(self):
        block = job_block(CI, "standard-linux")
        fetch = "cargo +nightly-2026-03-03 fetch --locked --manifest-path tests/fixtures/full-source-runtime/Cargo.toml"
        run = "cargo +nightly-2026-03-03 run --locked --offline --manifest-path tests/fixtures/full-source-runtime/Cargo.toml"
        self.assertLess(block.index(fetch), block.index("Build and exercise"))
        self.assertLess(block.index(fetch), block.index(run))
        self.assertIn("if: needs.impact.outputs.landing_candidate == 'true'", block)

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
        self.assertIn("Record exact native-plan checkout", native)
        self.assertIn('expected=$(git rev-parse "$VALIDATION_REF^{commit}")', native)
        self.assertIn('test "$actual" = "$expected"', native)
        self.assertIn('CARGO_BUILD_JOBS: "1"', native)
        self.assertIn('CARGO_PROFILE_DEV_DEBUG: "0"', native)
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', native)
        pair = "cargo +nightly-2026-03-03 test --locked -p mech-build --all-features --test registry_generated_project"
        wrapper = "cargo +nightly-2026-03-03 test --locked -p mech-build --test isolated_process -- --nocapture"
        planning_compile = "--test planning --no-run --message-format=json-render-diagnostics"
        planning_prepare = "--ignored --exact prepare_planning_owner_runners --test-threads=1 --nocapture"
        planning = "--stage 02-planning-execution"
        pruning = "cargo +nightly-2026-03-03 test --locked -p mech-build --all-features --test native_host_pruning"
        self.assertLess(native.index(pair), native.index(wrapper))
        self.assertLess(native.index(wrapper), native.index(planning_compile))
        self.assertLess(native.index(planning_compile), native.index(planning_prepare))
        self.assertLess(native.index(planning_prepare), native.index(planning))
        self.assertLess(native.index(planning), native.index(pruning))
        self.assertLess(native.index(pair), native.index(pruning))
        bounded = "cargo +nightly-2026-03-03 test --locked -p mech-build --all-features"
        self.assertEqual(native.count(bounded), 3)
        self.assertIn("cargo +nightly-2026-03-03 test --locked --offline -p mech-build --all-features", native)
        self.assertIn("--lib", native)
        self.assertIn("--test standard_host_source_planning", native)
        self.assertIn("Verify native build planning contracts", native)
        self.assertIn("witness=actor-retirement,artifact-closure", native)
        self.assertNotIn("--skip", native)
        for expensive_target in (
            "native_literal",
            "native_scalar",
            "native_representative_families",
            "native_output_seed_arities",
            "native_generated_arguments",
            "native_generated_end_to_end",
        ):
            self.assertNotIn(f"--test {expensive_target}", native)
        stages = (
            ("Verify native host pruning", pruning),
            ("Verify standard host source planning", "--test standard_host_source_planning"),
            ("Verify native host catalog", "python3 scripts/check-native-host-catalog.py"),
            ("Verify generated project determinism", "python3 scripts/check-generated-project-determinism.py"),
            ("Verify native application graphs", "python3 scripts/check-native-application-graphs.py"),
        )
        previous = -1
        for name, witness in stages:
            self.assertIn(f"- name: {name}", native)
            position = native.index(witness)
            self.assertGreater(position, previous)
            previous = position
        for script in ("check-native-host-catalog.py", "check-generated-project-determinism.py", "check-native-application-graphs.py"):
            self.assertIn(f"python3 scripts/{script}", native)
        self.assertNotIn("Verify bounded native planning and generation", native)
        self.assertIn("Verify native owner subprocess wrapper", native)
        self.assertIn(
            "cases=hang,parent-death,failed-parent-tree,nonzero,malformed-json", native
        )
        self.assertEqual(native.count("python3 scripts/run-native-plan-stage.py"), 12)
        self.assertNotIn("pipeline_status", native)
        self.assertIn("Retain native-plan diagnostics", native)
        self.assertIn("target/native-plan-logs", native)
        self.assertIn("target/bytecode-v1-fixtures/owner-runner/logs", native)
        self.assertIn("if: always()", native)
        self.assertIn("retention-days: 14", native)

    def test_planning_preparation_execution_and_deadlines_remain_separate(self):
        steps = job_steps(NATIVE, "native-plan")
        def named(name):
            return next(step for step in steps if f"- name: {name}\n" in step)
        compile_step = named("Compile native planning contracts without execution")
        prepare_step = named("Prepare exactly the planning owner profiles")
        execution_step = named("Verify native build planning contracts")
        self.assertIn("--record-test-executable planning", compile_step)
        self.assertIn("--no-run", compile_step)
        for step in (prepare_step, execution_step):
            self.assertIn("--test-executable-file target/native-plan-logs/planning-executable", step)
            self.assertNotIn("cargo +", step)
        self.assertIn("--ignored --exact prepare_planning_owner_runners", prepare_step)
        self.assertIn("&[OwnerProfile::Standard, OwnerProfile::Fixed]", NATIVE_PLANNING)
        self.assertIn('MECH_NATIVE_OWNER_REQUIRE_PREBUILT: "1"', execution_step)
        self.assertIn("--test-threads=1 --nocapture", execution_step)
        self.assertNotIn("--ignored", execution_step)
        self.assertNotIn("--exact", execution_step)
        self.assertNotIn("--skip", execution_step)
        build_timeout = int(re.search(r'MECH_NATIVE_OWNER_BUILD_TIMEOUT_SECS: "(\d+)"', prepare_step).group(1))
        prepare_timeout = int(re.search(r"--timeout-secs (\d+)", prepare_step).group(1))
        # Both serial profile builds and host discovery/reaping fit before the
        # outer boundary; that boundary also leaves room before Actions expiry.
        self.assertGreater(prepare_timeout, 2 * build_timeout + 2 * (30 + 10))
        for step in steps:
            if "python3 scripts/run-native-plan-stage.py" in step:
                command_timeouts = [int(value) for value in re.findall(r"--timeout-secs (\d+)", step)]
                actions_timeout = int(re.search(r"timeout-minutes: (\d+)", step).group(1)) * 60
                self.assertGreater(actions_timeout, sum(value + 30 + 5 + 15 for value in command_timeouts))
        for stage, checkpoint in (
            (compile_step, "Retain native planning compilation evidence"),
            (prepare_step, "Retain native planning preparation evidence"),
            (execution_step, "Retain native planning execution evidence before the next stage"),
        ):
            retained = steps[steps.index(stage) + 1]
            self.assertIn(checkpoint, retained)
            self.assertIn("if: always()", retained)
            self.assertIn("target/native-plan-logs", retained)
            self.assertIn("target/bytecode-v1-fixtures/owner-runner/logs", retained)
        self.assertIn("python3 -B scripts/tests/test_native_plan_stage.py", NATIVE)

    def test_native_metadata_subprocess_contracts_run_before_generation(self):
        steps = job_steps(NATIVE, "native-plan")
        name = "Verify native metadata subprocess contracts"
        metadata_steps = [step for step in steps if f"- name: {name}\n" in step]
        self.assertEqual(len(metadata_steps), 1)
        metadata = metadata_steps[0]
        self.assertRegex(metadata, r"(?m)^        timeout-minutes: 2$")
        self.assertRegex(
            metadata,
            r"(?m)^        run: python3 -B scripts/tests/test_native_application_graphs\.py$",
        )
        self.assertNotIn("continue-on-error", metadata)
        self.assertNotRegex(metadata, r"(?m)^        if:")
        position = steps.index(metadata)
        for generation_name in (
            "Verify registry projects and live shutdown first",
            "Verify generated project determinism",
            "Verify native application graphs",
        ):
            generation = next(
                step for step in steps if f"- name: {generation_name}\n" in step
            )
            self.assertLess(position, steps.index(generation))

    def test_native_generated_contracts_share_all_exact_case_identities(self):
        cases = literal_assignment(NATIVE_DETERMINISM, "EXPECTED_CASES")
        features = literal_assignment(NATIVE_APPLICATION_GRAPHS, "EXPECTED_FEATURES")
        graph_cases = literal_assignment(NATIVE_APPLICATION_GRAPHS, "EXPECTED_CASES")
        self.assertEqual(
            cases,
            {
                "literal": "generated_native_literal",
                "scalar": "generated_native_scalar",
                "unary": "generated_native_unary",
                "ternary": "generated_native_ternary",
                "quaternary": "generated_native_quaternary",
                "variadic": "generated_native_variadic",
                "integrity": "generated_native_integrity",
                "canonical-artifact-features": "generated_native_canonical_artifact_features",
                "fixed-matrix": "generated_native_fixed_matrix",
                "dynamic-matrix": "generated_native_dynamic_matrix",
                "cli": "generated_native_cli",
                "console": "generated_native_console",
                "time-once": "generated_native_time",
                "timer-once": "generated_native_timer",
                "scene": "generated_native_scene",
                "robot-arm": "generated_native_robot_arm",
            },
        )
        self.assertEqual(graph_cases, cases)
        self.assertEqual(set(cases.values()), set(features))
        canonical = features["generated_native_canonical_artifact_features"]
        self.assertEqual(
            canonical,
            {
                "mech-core": {"f32", "f64", "matrix2x3", "program", "u8"},
                "mech-engine": {"convert", "f32", "f64", "matrix2x3", "runtime", "u8"},
                "mech-runtime": {
                    "f32",
                    "f64",
                    "matrix2x3",
                    "resident-routing",
                    "runtime",
                    "string",
                    "u8",
                },
            },
        )
        self.assertIn('environment.pop("MECH_NATIVE_GENERATED_CASE", None)', NATIVE_APPLICATION_GRAPHS)
        self.assertIn('"--locked"', NATIVE_APPLICATION_GRAPHS)
        self.assertIn('"--offline"', NATIVE_APPLICATION_GRAPHS)
        self.assertIn("execute_streamed(", NATIVE_APPLICATION_GRAPHS)
        self.assertIn("PROJECT_MARKER", NATIVE_APPLICATION_GRAPHS)
        self.assertIn("case={case} profile={profile} stage=metadata", NATIVE_APPLICATION_GRAPHS)
        self.assertIn("environment.pop(CASE_SELECTOR_ENV, None)", NATIVE_DETERMINISM)
        self.assertIn("tempfile.TemporaryDirectory", NATIVE_DETERMINISM)
        self.assertIn('generate(projects_root, "first")', NATIVE_DETERMINISM)
        self.assertIn('generate(projects_root, "second")', NATIVE_DETERMINISM)

    def test_generated_program_selectors_are_live_and_fail_closed(self):
        generated_commands = re.findall(
            r"cargo \+nightly-2026-03-03 test --locked --offline "
            r"-p mech-build --features full-hosts\s*(?:\\\s*)?"
            r"--test native_generated_end_to_end",
            FULL + NATIVE_APPLICATIONS,
        )
        self.assertEqual(len(generated_commands), 5)

        windows = job_block(FULL, "native-windows")
        self.assertNotIn("actor-alpha", windows)
        self.assertNotIn("actor-beta", windows)
        self.assertIn(
            '$cases = @("scalar", "fixed-matrix", "dynamic-matrix", "cli", "console", "time-once")',
            windows,
        )
        self.assertIn("Validate Windows owner-process containment", windows)
        self.assertIn("--test isolated_process -- --nocapture", windows)
        qualification = "Compile required Windows owner-helper consumers"
        self.assertIn(qualification, windows)
        for command in (
            "cargo +nightly-2026-03-03 test --locked --offline -p mech-build --test isolated_process --no-run",
            "cargo +nightly-2026-03-03 test --locked --offline -p mech-build --features full-hosts --test native_generated_end_to_end --no-run",
        ):
            self.assertIn(command, windows)
            self.assertLess(windows.index(command), windows.index("Validate Windows owner-process containment"))
            self.assertLess(windows.index(command), windows.index("Build selected native fixture families"))
        self.assertIn('#![cfg(feature = "full-hosts")]', NATIVE_GENERATED_END_TO_END)
        self.assertIn("assert_eq!(", NATIVE_GENERATED_END_TO_END)
        self.assertIn("matched, requested", NATIVE_GENERATED_END_TO_END)
        self.assertIn(
            "production_builder_rejects_actor_bootstrap_before_planning",
            NATIVE_PLANNING,
        )
        native = job_block(NATIVE, "native-plan")
        self.assertIn("--test planning --no-run --message-format=json-render-diagnostics", native)
        self.assertIn("--stage 02-planning-execution", native)

        engine = job_block(FULL, "native-engine")
        self.assertIn("uses: ./.github/actions/native-applications", engine)
        self.assertIn("suite: engine", engine)
        grouped = (
            "cargo +nightly-2026-03-03 test --locked --offline "
            "-p mech-build --features full-hosts"
        )
        application_commands = " ".join(NATIVE_APPLICATIONS.replace("\\\n", " ").split())
        self.assertEqual(application_commands.count(grouped), 4)
        for target in (
            "native_literal",
            "native_scalar",
            "native_representative_families",
            "native_output_seed_arities",
            "native_generated_arguments",
        ):
            self.assertIn(f"--test {target}", application_commands)

        hosted = job_block(FULL, "native-hosted")
        self.assertIn("uses: ./.github/actions/native-applications", hosted)
        self.assertIn("suite: hosted", hosted)
        self.assertIn(
            f"{grouped} --test native_cli_hosted",
            application_commands,
        )
        self.assertIn(
            "cargo +nightly-2026-03-03 test --locked --offline "
            "-p mech-build --features full-hosts --test native_generated_end_to_end",
            application_commands,
        )
        self.assertIn("MECH_NATIVE_GENERATED_CASE: console", NATIVE_APPLICATIONS)
        self.assertIn("--lib cli::app::tests::build_", NATIVE_APPLICATIONS)
        self.assertIn("--test mech_build", NATIVE_APPLICATIONS)

    def test_affected_native_commands_retain_each_stage_and_reuse_same_recipe(self):
        stages = re.split(r"(?m)^    - name: ", NATIVE_APPLICATIONS)[1:]
        bounded = [stage for stage in stages if "python3 scripts/run-native-plan-stage.py" in stage]
        self.assertEqual(len(bounded), 6)
        for stage in bounded:
            following = stages[stages.index(stage) + 1]
            self.assertIn("if: always()", following)
            self.assertIn("uses: actions/upload-artifact@v4", following)
            self.assertIn("target/native-plan-logs", following)
            self.assertIn("target/bytecode-v1-fixtures/owner-runner/logs", following)
        self.assertNotIn("--skip", NATIVE_APPLICATIONS)
        self.assertNotIn("continue-on-error", NATIVE_APPLICATIONS)
        self.assertIn("branches: [codex/r21-native-stabilization]", NATIVE_STABILIZATION)
        self.assertIn("suite: [engine, hosted]", NATIVE_STABILIZATION)
        self.assertEqual(NATIVE_STABILIZATION.count("uses: ./.github/actions/native-applications"), 2)
        self.assertIn("phase: cold", NATIVE_STABILIZATION)
        self.assertIn("phase: reuse", NATIVE_STABILIZATION)
        self.assertIn("verify_reuse: true", NATIVE_STABILIZATION)
        for job in ("native-plan", "applications"):
            self.assertIn("needs: harness", job_block(NATIVE_STABILIZATION, job))
        self.assertIn("python3 scripts/probe-unix-kill-targeting.py", NATIVE_STABILIZATION)
        reuse = next(step for step in job_steps(NATIVE, "native-plan")
                     if "- name: Verify fresh-process owner reuse and planning execution" in step)
        self.assertIn("if: inputs.verify_reuse", reuse)
        self.assertIn('MECH_NATIVE_OWNER_REQUIRE_PREBUILT: "1"', reuse)
        self.assertIn("--stage 02-planning-reuse-execution", reuse)

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

    def test_docs_only_gate_accepts_successful_browser_skip_verification(self):
        block = job_block(CI, "pr-gate")
        script = textwrap.dedent(block.split("        run: |\n", 1)[1])
        environment = dict.fromkeys(re.findall(r"^          ([A-Z0-9_]+):", block, re.M), "skipped")
        environment.update(DOCS_ONLY="true", FULL_REQUIRED="false", IMPACT_RESULT="success", BROWSER_RESULT="success")
        def accepts(**changes):
            return subprocess.run(["/bin/bash", "-e", "-c", script],
                env=environment | changes, capture_output=True).returncode == 0
        self.assertTrue(accepts())
        for result in ("failure", "cancelled", "skipped", ""):
            with self.subTest(result=result):
                self.assertFalse(accepts(BROWSER_RESULT=result))
        self.assertFalse(accepts(STATIC_RESULT="failure"))
        self.assertFalse(accepts(FULL_REQUIRED="true"))
        self.assertTrue(accepts(FULL_REQUIRED="true", FULL_RESULT="success", NATIVE_PLAN_RESULT="success"))

    def test_cancelled_impact_skips_aggregate_gates(self):
        guard = "if: always() && needs.impact.result != 'cancelled'"
        for job in ("browser-compute-canary", "browser-canary", "pr-gate"):
            with self.subTest(job=job):
                self.assertIn(guard, job_block(CI, job))

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
        compute_build = job_block(CI, "browser-compute-build")
        compute_smoke = job_block(CI, "browser-compute-smoke")
        ekf_cpu = job_block(CI, "browser-ekf-cpu")
        ekf_wgpu = job_block(CI, "browser-ekf-wgpu")
        ekf_parity = job_block(CI, "browser-ekf-parity")
        compute = job_block(CI, "browser-compute-canary")
        aggregate = job_block(CI, "browser-canary")

        self.assertIn("Build standard WASM and the standard server", standard)
        self.assertIn("Verify resident rendering without browser errors", standard)
        self.assertNotIn("Verify N-body physics against independent references", standard)
        self.assertIn("Verify N-body physics against independent references", nbody)
        self.assertIn("Build mixed compute WASM and refresh the server", compute_build)
        self.assertIn("--profile browser-compute-canary", compute_build)
        self.assertIn("browser-compute-canary-build", compute_build)
        self.assertIn("Verify report-only particle WebGPU execution", compute_smoke)
        self.assertIn("Verify scalar EKF rendering", ekf_cpu)
        self.assertIn("Verify WebGPU EKF rendering", ekf_wgpu)
        self.assertIn("scripts/check-ekf-browser-results.py", ekf_parity)
        for dependency in (
            "browser-compute-build",
            "browser-compute-smoke",
            "browser-ekf-cpu",
            "browser-ekf-wgpu",
            "browser-ekf-parity",
        ):
            self.assertIn(f"- {dependency}", compute)
        for result in ("BUILD", "SMOKE", "CPU", "WGPU", "PARITY"):
            self.assertIn(f'test "${result}_RESULT" = success', compute)
        self.assertIn("Smoke test rich served documents before full validation", standard)
        self.assertIn("smoke-served-rich-document-browser.sh", standard)
        project_browser = job_block(FULL, "project-browser")
        self.assertIn("!inputs.rich_browser_smoke_in_caller", project_browser)
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
        self.assertIn("smoke-gpu-particles-browser.py", compute_smoke)
        self.assertIn("smoke-served-resident-ekf-browser.sh", ekf_cpu)
        self.assertIn("smoke-served-resident-ekf-browser.sh", ekf_wgpu)
        self.assertNotIn("smoke-served-resident-nbody-browser.sh", compute)

    def test_long_browser_canaries_use_progress_watchdogs_with_hard_caps(self):
        static_architecture = job_block(CI, "static-architecture")
        self.assertIn(
            "tests.browser.harness.test_completion_server", static_architecture
        )
        self.assertIn("if message in progress:", BROWSER_HARNESS)
        self.assertIn("deadline = min(deadline, max_deadline)", BROWSER_HARNESS)
        self.assertIn("last progress", BROWSER_HARNESS)
        self.assertIn("no progress beacon was received", BROWSER_HARNESS)
        self.assertIn('progress=("nbody-progress",)', NBODY_BROWSER)
        self.assertIn("max_timeout=600", NBODY_BROWSER)
        self.assertIn('progress=("ekf-terminal-progress",)', EKF_BROWSER)
        self.assertIn('progress=("ekf-progress",)', EKF_BROWSER)
        self.assertIn("max_timeout=900", EKF_BROWSER)
        for milestone in (
            "document-ready",
            "first-compute-submit",
            "first-compute-completion",
            "compute-checkpoint-",
            "continuity-post-edit-turn-complete",
            "parity-turn-complete",
            "continuity-replacement-accepted",
            "incompatible-replacement-accepted",
            "shutdown-completed",
        ):
            self.assertIn(milestone, EKF_BROWSER)

    def test_ekf_failure_diagnostics_are_not_numerical_evidence(self):
        for backend, label in (("cpu", "scalar"), ("wgpu", "WebGPU")):
            with self.subTest(backend=backend):
                steps = job_steps(CI, f"browser-ekf-{backend}")
                evidence = next(
                    step for step in steps
                    if f"name: Upload {label} EKF evidence" in step
                )
                diagnostics = next(
                    step for step in steps
                    if f"name: Retain {label} EKF failure diagnostics" in step
                )
                self.assertNotRegex(evidence, r"(?m)^        if:")
                self.assertIn(f"name: browser-ekf-{backend}-results\n", evidence)
                self.assertIn("if-no-files-found: error", evidence)
                self.assertIn("if: always()", diagnostics)
                self.assertIn("uses: actions/upload-artifact@v4", diagnostics)
                self.assertIn(f"name: browser-ekf-{backend}-diagnostics-", diagnostics)
                self.assertIn("path: target/served-resident-ekf-browser.*", diagnostics)
                self.assertNotIn("-results", diagnostics)
                self.assertGreater(steps.index(diagnostics), steps.index(evidence))
        for field in (
            "mechGpuBindingInventory", "mechGpuRequiredStorageBindings",
            "mechGpuSupportedStorageBindings", "mechGpuBridgeError",
        ):
            self.assertIn(f"root.dataset.{field}", EKF_BROWSER)
            self.assertIn(f'dataset.get("{field}", "")', EKF_BROWSER)
        self.assertIn('2>"$harness_log"', EKF_BROWSER)

    def test_ekf_terminal_probe_reaps_its_browser_before_numerical_launch(self):
        terminal = EKF_BROWSER.index('"terminal compute submission proof failed:')
        close = EKF_BROWSER.index("            browser.close()", terminal)
        restart = EKF_BROWSER.index("            browser = ChromeSession", close)
        numerical = EKF_BROWSER.index('"the fresh numeric EKF document to commit"', restart)
        self.assertLess(terminal, close)
        self.assertLess(close, restart)
        self.assertLess(restart, numerical)
        self.assertIn('"terminal-probe.json"', EKF_BROWSER[terminal:numerical])
        self.assertIn('"terminal.chrome.stderr"', EKF_BROWSER[terminal:numerical])
        self.assertIn('"EKF_TERMINAL_SUBMIT"', EKF_BROWSER[terminal:numerical])

    def test_ekf_cleanup_keeps_failed_artifacts_and_preserves_exit_status(self):
        match = re.search(
            r"(?ms)^cleanup\(\) \{\n.*?^\}\n(?=trap cleanup EXIT)", EKF_BROWSER,
        )
        self.assertIsNotNone(match)
        capability_error = (
            "region ekf-batch requires 12 for maxStorageBuffersPerShaderStage, "
            "but this adapter supports 10"
        )
        inventory = json.dumps([{"binding": 0, "name": "state_read", "access": "read"}])
        for status, software_adapter in ((0, True), (0, False), (27, True), (27, False)):
            with self.subTest(status=status, software_adapter=software_adapter), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                project = directory / "project"
                browser = directory / "browser"
                profile = browser / "chrome-profile"
                project.mkdir()
                profile.mkdir(parents=True)
                (profile / "private-profile-data").write_text("discard profile")
                (browser / "server.log").write_text("server diagnostics\n")
                (browser / "chrome.stderr").write_text("adapter diagnostics\n")
                (browser / "harness.stderr").write_text("bridge creation failed\n")
                (browser / "chrome.dom").write_text(
                    '<html data-mech-compute-dispatches="0" '
                    f'data-mech-document-error="{html.escape(capability_error)}" '
                    f'data-mech-gpu-binding-inventory="{html.escape(inventory)}">'
                    "<head></head><body></body></html>\n"
                )
                script = "\n".join((
                    "set -euo pipefail",
                    f"project_dir={shlex.quote(str(project))}",
                    f"browser_dir={shlex.quote(str(browser))}",
                    f"chrome_profile={shlex.quote(str(profile))}",
                    "server_pid=''",
                    "compute_backend=wgpu",
                    "filter_count=1000",
                    "continuity_edit=false",
                    "terminal_submit_probe=false",
                    f"software_adapter={str(software_adapter).lower()}",
                    match.group(),
                    "trap cleanup EXIT",
                    f"exit {status}",
                ))
                result = subprocess.run(
                    ["bash", "--noprofile", "--norc", "-c", script], cwd=ROOT,
                    text=True, capture_output=True, timeout=30,
                )
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertFalse(project.exists())
                if status == 0:
                    self.assertFalse(browser.exists())
                    continue
                self.assertFalse(profile.exists())
                self.assertIn(f"Retained EKF failure diagnostics: {browser}", result.stderr)
                self.assertTrue((browser / "chrome.stderr").exists())
                self.assertTrue((browser / "harness.stderr").exists())
                failure = json.loads((browser / "failure.json").read_text())
                self.assertEqual(failure["outcome"], "failed")
                self.assertEqual(failure["exit_status"], status)
                self.assertEqual(failure["requested_backend"], "wgpu")
                self.assertEqual(failure["browser_software_adapter_requested"], software_adapter)
                self.assertRegex(failure["revision"], r"^[0-9a-f]{40}$")
                self.assertEqual(
                    failure["dataset"]["data-mech-document-error"], capability_error,
                )
                self.assertEqual(
                    failure["dataset"]["data-mech-gpu-binding-inventory"], inventory,
                )
                self.assertEqual(failure["dataset"]["data-mech-compute-dispatches"], "0")
                self.assertNotIn("output", failure)
                self.assertFalse((directory / "ekf-wgpu-no-edit.json").exists())

    def test_ekf_adapter_selection_is_validated_before_product_startup(self):
        match = re.search(
            r'(?ms)^software_adapter="\$\{MECH_BROWSER_SOFTWARE_ADAPTER:-true\}"\n.*?^esac\n',
            EKF_BROWSER,
        )
        self.assertIsNotNone(match)
        self.assertLess(match.start(), EKF_BROWSER.index('project_dir="$(mktemp'))
        for requested, expected in ((None, "true"), ("true", "true"), ("false", "false"), ("typo", None)):
            with self.subTest(requested=requested):
                environment = os.environ.copy()
                environment.pop("MECH_BROWSER_SOFTWARE_ADAPTER", None)
                if requested is not None:
                    environment["MECH_BROWSER_SOFTWARE_ADAPTER"] = requested
                result = subprocess.run(
                    ["bash", "--noprofile", "--norc", "-c", "set -euo pipefail\n" + match.group() + 'printf "%s" "$software_adapter"'],
                    env=environment, text=True, capture_output=True, timeout=10,
                )
                if expected is None:
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("must be true or false", result.stderr)
                    self.assertEqual(result.stdout, "")
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout, expected)

    def test_engine_owner_runs_source_semantics_before_full_validation(self):
        owners = (ROOT / ".github/ci/owners.toml").read_text(encoding="utf-8")
        engine = owners.split("[owners.mech-engine]", 1)[1].split("\n[owners.", 1)[0]
        self.assertIn('"--test", "canonical_source_semantics"', engine)

    def test_runtime_compute_contracts_execute_mixed_source_tests(self):
        cargo_language = job_block(FULL, "cargo-language")
        self.assertIn(
            "Verify mixed-source compute admission contracts", cargo_language
        )
        self.assertIn(
            "--no-default-features --features full_compiler,watcher,compute",
            cargo_language,
        )
        self.assertIn("MIXED_TEST_LIST=$(cargo", cargo_language)
        self.assertIn("MIXED_TEST_COUNT=$(", cargo_language)
        self.assertIn("/: test$/", cargo_language)
        self.assertIn('test "$MIXED_TEST_COUNT" -gt 0', cargo_language)
        self.assertIn("--lib mixed_ -- --nocapture", cargo_language)

    def test_pr_full_validation_receives_exact_head(self):
        block = job_block(CI, "full-validation")
        self.assertIn(
            "validation_ref: ${{ github.event.pull_request.head.sha }}", block
        )
        self.assertIn(
            "rich_browser_smoke_in_caller: ${{ needs.impact.outputs.browser_canary_required == 'true' }}",
            block,
        )

    def test_windows_cache_never_skips_the_current_product_or_source_smoke(self):
        steps = job_steps(CI, "standard-windows")
        cache_index = next(
            index for index, step in enumerate(steps)
            if "Mozilla-Actions/sccache-action@" in step
        )
        toolchain_index = next(
            index for index, step in enumerate(steps)
            if "rustup default nightly-2026-03-03" in step
        )
        build_index = next(
            index for index, step in enumerate(steps)
            if "./scripts/build-mech.ps1" in step
        )
        self.assertLess(toolchain_index, cache_index)
        self.assertLess(cache_index, build_index)
        build = steps[build_index]
        self.assertNotRegex(build, r"(?m)^\s+if:")
        self.assertNotIn("continue-on-error", build)
        self.assertIn("target/release/mech.exe --version", build)
        self.assertIn(
            "target/release/mech.exe run tests/fixtures/standard-resident-scalar.mec",
            build,
        )
        producer = (ROOT / "scripts/build-mech.ps1").read_text(encoding="utf-8")
        wasm = producer.index("python scripts/build-wasm.py --profile browser-compute")
        native = producer.index("cargo build --locked --release --features compute_backends_native")
        self.assertLess(wasm, native)
        self.assertIn("Remove-Item $nativeArtifact -Force", producer)

    def test_windows_compiler_cache_preserves_failures_and_partial_dependency_reuse(self):
        block = job_block(CI, "standard-windows")
        self.assertIn("CARGO_INCREMENTAL: 0", block)
        self.assertIn("RUSTC_WRAPPER: sccache", block)
        self.assertIn('SCCACHE_GHA_ENABLED: "true"', block)
        # Cache-server I/O failures fall back to rustc, never suppress its errors.
        self.assertIn('SCCACHE_IGNORE_SERVER_IO_ERROR: "1"', block)
        dependencies = next(
            step for step in job_steps(CI, "standard-windows")
            if "Swatinem/rust-cache@" in step
        )
        self.assertIn('cache-on-failure: "true"', dependencies)
        self.assertIn('cache-workspace-crates: "false"', dependencies)
        self.assertNotIn("continue-on-error", block)

    def test_standard_browser_cache_keeps_current_build_and_every_smoke(self):
        block = job_block(CI, "browser-standard-canary")
        steps = job_steps(CI, "browser-standard-canary")
        toolchain = next(index for index, step in enumerate(steps)
                         if "rustup default nightly-2026-03-03" in step)
        cache = next(index for index, step in enumerate(steps)
                     if "Mozilla-Actions/sccache-action@" in step)
        build = next(index for index, step in enumerate(steps)
                     if "Build standard WASM and the standard server" in step)
        self.assertLess(toolchain, cache)
        self.assertLess(cache, build)
        self.assertIn("RUSTC_WRAPPER: sccache", block)
        self.assertIn('SCCACHE_GHA_ENABLED: "true"', block)
        self.assertIn('SCCACHE_IGNORE_SERVER_IO_ERROR: "1"', block)
        dependencies = next(step for step in steps if "Swatinem/rust-cache@" in step)
        self.assertIn('cache-on-failure: "true"', dependencies)
        self.assertIn('cache-workspace-crates: "false"', dependencies)
        for command in (
            "python3 scripts/build-wasm.py --profile browser",
            "cargo build --locked --bin mech",
            "bash scripts/smoke-served-resident-nbody-browser.sh",
            "bash scripts/smoke-served-rich-document-browser.sh",
            "python3 -B scripts/smoke-browser-document-lifecycle.py",
        ):
            step = next(step for step in steps if command in step)
            # A cache hit may reuse compiler work, never skip product verification.
            self.assertNotRegex(step, r"(?m)^\s+if:")
        self.assertNotIn("continue-on-error", block)
        build_step = steps[build]
        self.assertLess(build_step.index("scripts/build-wasm.py"),
                        build_step.index("cargo build --locked --bin mech"))
        # 30 minutes cancelled a healthy cold build before lifecycle validation.
        timeout = int(re.search(r"timeout-minutes: (\d+)", block).group(1))
        self.assertGreater(timeout, 30)
        self.assertLessEqual(timeout, 60)

    def test_standard_browser_keeps_failure_progress_without_profile_uploads(self):
        steps = job_steps(CI, "browser-standard-canary")
        evidence = next(step for step in steps
                        if "Upload standard browser failure diagnostics" in step)
        self.assertIn("failure() || cancelled()", evidence)
        self.assertIn("target/served-rich-document.*/progress.log", evidence)
        self.assertIn("target/served-rich-document.*/**/chrome.dom", evidence)
        self.assertIn("target/served-rich-document.*/**/server.log", evidence)
        self.assertIn("target/browser-document-lifecycle/*.json", evidence)
        self.assertNotIn("chrome-profile", evidence)
        cleanup = next(step for step in steps if "Remove generated browser package" in step)
        self.assertLess(steps.index(evidence), steps.index(cleanup))
        smoke = (ROOT / "scripts/smoke-served-rich-document-browser.sh").read_text()
        self.assertIn('tee -a "$work_dir/progress.log"', smoke)
        for function in ("run_case", "run_configured_case"):
            body = smoke.split(f"{function}() {{", 1)[1].split("\n}", 1)[0]
            self.assertIn('report_case_progress "$label" started', body)
            self.assertIn('report_case_progress "$label" passed', body)
            self.assertLess(body.index("run_browser_case"),
                            body.index('report_case_progress "$label" passed'))

    def test_full_validation_stops_on_cancellation_and_keeps_selected_dependency_gates(self):
        block = job_block(CI, "full-validation")
        condition = re.search(r"(?s)    if: >-\n\s*\$\{\{(.*?)\}\}", block).group(1)
        self.assertIn("!cancelled()", condition)

        def selected(cancelled=False, **overrides):
            values = {
                "needs.impact.outputs.full_validation_required": "true",
                "needs.impact.outputs.docs_only": "false",
                "needs.static-contracts.result": "success",
                "needs.standard-linux.result": "success",
                "needs.standard-windows.result": "success",
                "needs.changed-owner-tests.result": "success",
                "needs.browser-canary.result": "success",
            }
            values.update(overrides)
            expression = re.sub(
                r"needs\.[a-z-]+\.(?:outputs\.[a-z_]+|result)",
                lambda match: repr(values[match.group()]), condition,
            )
            expression = expression.replace("cancelled()", repr(cancelled)).replace("always()", "True")
            expression = expression.replace("&&", " and ").replace("||", " or ")
            expression = re.sub(r"!(?!=)", " not ", expression)
            expression = " ".join(expression.split())
            return eval(expression, {"__builtins__": {}}, {})

        self.assertTrue(selected())
        self.assertFalse(selected(cancelled=True))
        self.assertFalse(selected(**{"needs.impact.outputs.full_validation_required": "false"}))
        skipped_optional = {
            "needs.changed-owner-tests.result": "skipped",
            "needs.browser-canary.result": "skipped",
        }
        self.assertTrue(selected(**skipped_optional))
        self.assertFalse(selected(cancelled=True, **skipped_optional))
        skipped_docs = {
            "needs.impact.outputs.docs_only": "true",
            **{f"needs.{job}.result": "skipped" for job in (
                "static-contracts", "standard-linux", "standard-windows",
                "changed-owner-tests", "browser-canary",
            )},
        }
        self.assertTrue(selected(**skipped_docs))
        self.assertFalse(selected(cancelled=True, **skipped_docs))
        for job in ("static-contracts", "standard-linux", "standard-windows",
                    "changed-owner-tests", "browser-canary"):
            for result in ("failure", "cancelled"):
                with self.subTest(job=job, result=result):
                    self.assertFalse(selected(**{f"needs.{job}.result": result}))
        for job in ("static-contracts", "standard-linux", "standard-windows"):
            with self.subTest(job=job, result="skipped"):
                self.assertFalse(selected(**{f"needs.{job}.result": "skipped"}))

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

    def test_normal_ci_checkout_roles_are_explicit_and_recorded(self):
        self.assertEqual(
            checkout_jobs(CI),
            INTEGRATION_CHECKOUT_JOBS | EXACT_HEAD_CHECKOUT_JOBS,
        )
        roles = (
            (
                INTEGRATION_CHECKOUT_JOBS,
                "integration-merge",
                "${{ github.sha }}",
            ),
            (
                EXACT_HEAD_CHECKOUT_JOBS,
                "exact-head",
                "${{ github.event.pull_request.head.sha }}",
            ),
        )
        for jobs, validation_kind, requested_ref in roles:
            for job in jobs:
                with self.subTest(job=job, validation_kind=validation_kind):
                    steps = job_steps(CI, job)
                    checkout_indexes = [
                        index
                        for index, step in enumerate(steps)
                        if "uses: actions/checkout@" in step
                    ]
                    self.assertEqual(checkout_indexes, [0])
                    checkout = steps[0]
                    recorder = steps[1]
                    self.assertIn("id: checkout", checkout)
                    self.assertIn(f"ref: {requested_ref}", checkout)
                    self.assertIn(f"VALIDATION_KIND: {validation_kind}", recorder)
                    self.assertIn(f"REQUESTED_REF: {requested_ref}", recorder)
                    self.assertIn(
                        "CHECKOUT_REF: ${{ steps.checkout.outputs.ref }}", recorder
                    )
                    self.assertIn(
                        "CHECKOUT_SHA: ${{ steps.checkout.outputs.commit }}",
                        recorder,
                    )
                    self.assertIn(
                        'run: bash --noprofile --norc -euo pipefail -c "$RECORD_CHECKOUT_IDENTITY"',
                        recorder,
                    )
                    self.assertIn("shell: bash", recorder)
                    self.assertNotIn("uses: ./.github/actions/", recorder)

    def test_full_ci_checkouts_are_exact_ref_and_recorded_immediately(self):
        jobs = checkout_jobs(FULL)
        self.assertEqual(len(jobs), 37)
        for job in jobs:
            with self.subTest(job=job):
                steps = job_steps(FULL, job)
                checkout_indexes = [
                    index
                    for index, step in enumerate(steps)
                    if "uses: actions/checkout@" in step
                ]
                self.assertEqual(checkout_indexes, [0])
                checkout = steps[0]
                recorder = steps[1]
                self.assertIn("id: checkout", checkout)
                self.assertIn(FULL_CHECKOUT_REF, checkout)
                self.assertIn("VALIDATION_KIND: exact-ref", recorder)
                self.assertIn(
                    "REQUESTED_REF: ${{ inputs.validation_ref || github.sha }}",
                    recorder,
                )
                self.assertIn(
                    "CHECKOUT_REF: ${{ steps.checkout.outputs.ref }}", recorder
                )
                self.assertIn(
                    "CHECKOUT_SHA: ${{ steps.checkout.outputs.commit }}", recorder
                )
                self.assertIn(
                    'run: bash --noprofile --norc -euo pipefail -c "$RECORD_CHECKOUT_IDENTITY"',
                    recorder,
                )
                self.assertIn("shell: bash", recorder)
                self.assertNotIn("uses: ./.github/actions/", recorder)

    def test_checkout_identity_recorder_is_fail_closed_and_auditable(self):
        self.assertEqual(CHECKOUT_IDENTITY, workflow_checkout_identity(FULL))
        for validation_kind in ("integration-merge", "exact-head", "exact-ref"):
            self.assertIn(validation_kind, CHECKOUT_IDENTITY)
        self.assertIn("actual_sha=$(git rev-parse HEAD)", CHECKOUT_IDENTITY)
        self.assertIn('test -n "$REQUESTED_REF"', CHECKOUT_IDENTITY)
        self.assertIn('test -n "$CHECKOUT_SHA"', CHECKOUT_IDENTITY)
        self.assertIn(
            'test "$actual_sha" = "$CHECKOUT_SHA"', CHECKOUT_IDENTITY
        )
        self.assertIn(
            'test "$actual_sha" = "$requested_sha"', CHECKOUT_IDENTITY
        )
        self.assertIn('tee -a "$GITHUB_STEP_SUMMARY"', CHECKOUT_IDENTITY)
        for field in (
            "validation_kind",
            "requested_ref",
            "checkout_ref",
            "checkout_sha",
            "actual_sha",
        ):
            self.assertIn(f'echo "{field}=', CHECKOUT_IDENTITY)
        self.assertNotIn("continue-on-error", CHECKOUT_IDENTITY)

    def test_checkout_identity_recorder_works_for_a_tree_without_the_action(self):
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=repository, check=True)
            (repository / "historical.txt").write_text("historical\n", encoding="utf-8")
            subprocess.run(["git", "add", "historical.txt"], cwd=repository, check=True)
            commit_env = os.environ.copy()
            commit_env.update(
                {
                    "GIT_AUTHOR_NAME": "CI contract",
                    "GIT_AUTHOR_EMAIL": "ci-contract@example.invalid",
                    "GIT_COMMITTER_NAME": "CI contract",
                    "GIT_COMMITTER_EMAIL": "ci-contract@example.invalid",
                }
            )
            subprocess.run(
                ["git", "commit", "-q", "-m", "historical"],
                cwd=repository,
                env=commit_env,
                check=True,
            )
            sha = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repository, text=True
            ).strip()
            summary = repository / "summary.md"
            verifier_env = os.environ.copy()
            verifier_env.update(
                {
                    "VALIDATION_KIND": "exact-ref",
                    "REQUESTED_REF": sha,
                    "CHECKOUT_REF": "",
                    "CHECKOUT_SHA": sha,
                    "GITHUB_STEP_SUMMARY": str(summary),
                }
            )
            command = [
                "bash",
                "--noprofile",
                "--norc",
                "-euo",
                "pipefail",
                "-c",
                CHECKOUT_IDENTITY,
            ]
            subprocess.run(command, cwd=repository, env=verifier_env, check=True)
            self.assertIn(f"actual_sha={sha}", summary.read_text(encoding="utf-8"))

            mismatched = verifier_env | {"REQUESTED_REF": "0" * len(sha)}
            self.assertNotEqual(
                subprocess.run(command, cwd=repository, env=mismatched).returncode,
                0,
            )
            unsupported = verifier_env | {"VALIDATION_KIND": "unknown"}
            self.assertNotEqual(
                subprocess.run(command, cwd=repository, env=unsupported).returncode,
                0,
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

    def test_dynamic_modules_prefetches_before_offline_native_build(self):
        block = job_block(FULL, "dynamic-modules")
        fetch = "cargo fetch --locked"
        smoke = "bash scripts/test-dynamic-modules.sh"
        self.assertIn(fetch, block)
        self.assertLess(block.index(fetch), block.index(smoke))

    def test_language_census_prefetches_before_offline_metadata_tests(self):
        block = job_block(FULL, "cargo-language")
        fetch = "cargo fetch --locked"
        self.assertIn(fetch, block)
        for profile in ("full", "base"):
            command = (
                "cargo +nightly-2026-03-03 test --locked -p mech-syntax "
                f"--tests --no-default-features --features {profile}"
            )
            with self.subTest(profile=profile):
                self.assertIn(command, block)
                self.assertLess(block.index(fetch), block.index(command))
        self.assertNotIn("continue-on-error", block)

    def test_syntax_grammar_step_uses_current_targets_and_features(self):
        block = job_block(FULL, "cargo-language")
        grammar = block.split("- name: Run syntax grammar suites", 1)[1].split("- name:", 1)[0]
        self.assertNotRegex(grammar, r"--test\s+grammar_conformance\b")
        for target in (
            "canonical_port_registry", "canonical_rule_registry",
            "canonical_recursive_core_inventory", "canonical_recursive_core_schema",
            "canonical_grammar", "canonical_mechdown_closed_rules",
            "canonical_phase_2i_certification", "canonical_phase_2i_recovery",
        ):
            self.assertIn(f"--test {target}", grammar)
            self.assertTrue((ROOT / f"src/syntax/tests/{target}.rs").is_file())
        for selected in re.findall(r'--features\s+(?:"([^"]*)"|\'(.*?)\'|(\S+))', grammar):
            features = next(value for value in selected if value)
            self.assertNotIn("mechdown", features.replace(",", " ").split())
            self.assertNotIn("formatter", features.replace(",", " ").split())

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
