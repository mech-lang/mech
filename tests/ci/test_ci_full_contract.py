#!/usr/bin/env python3

import ast
import html
import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

from scripts.rust_source import rust_code


ROOT = Path(__file__).resolve().parents[2]
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
FULL = (ROOT / ".github/workflows/ci-full.yml").read_text(encoding="utf-8")
NATIVE = (ROOT / ".github/workflows/ci-native-plan.yml").read_text(encoding="utf-8")
NATIVE_APPLICATIONS = (ROOT / ".github/actions/native-applications/action.yml").read_text(encoding="utf-8")
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
    def test_benchmark_status_keeps_unique_keys_and_both_cli_records(self):
        def unique_object(pairs):
            result = {}
            for key, value in pairs:
                if key in result:
                    raise ValueError(f"duplicate validation key: {key}")
                result[key] = value
            return result

        source = (ROOT / "docs/validation/benchmark-compiler-status-v1.json").read_text()
        document = json.loads(source, object_pairs_hook=unique_object)
        qualification = document["cli_payload_continuation"]
        for key in ("qualified_source_commit", "result", "remaining", "ci_wiring_source_commit"):
            self.assertIn(key, qualification)
        history = document["cli_payload_publication_history"]
        self.assertEqual(history["pull_request"], "https://github.com/mech-lang/mech/pull/903")
        self.assertEqual(history["status"], "INHERITED_PR903_IMPLEMENTATION_HISTORICAL_PUBLICATION_RECEIPT")
        self.assertNotIn("NOT composed", history["scope"])
        # Exercise real JSON parsing, including nested objects, rather than a
        # textual key count that could confuse strings with object members.
        mutants = (
            source.replace('"cli_payload_continuation": {',
                           '"cli_payload_continuation": {}, "cli_payload_continuation": {', 1),
            '{"outer": {"qualified_source_commit": "first", "qualified_source_commit": "lost"}}',
        )
        for mutant in mutants:
            with self.subTest(mutant=mutant[:100]):
                with self.assertRaisesRegex(ValueError, "duplicate validation key"):
                    json.loads(mutant, object_pairs_hook=unique_object)

    def test_linked_applications_have_an_independent_required_cold_build_budget(self):
        applications = job_block(FULL, "canonical-applications")
        linked = job_block(FULL, "canonical-linked-applications")
        self.assertEqual(int(re.search(r"timeout-minutes: (\d+)", applications).group(1)), 90)
        self.assertEqual(int(re.search(r"timeout-minutes: (\d+)", linked).group(1)), 240)
        self.assertEqual(applications.splitlines()[0], linked.splitlines()[0])
        for setting in (
            "RUSTUP_TOOLCHAIN: nightly-2026-03-03",
            "CARGO_INCREMENTAL: 0", 'CARGO_PROFILE_DEV_DEBUG: "0"',
            'CARGO_PROFILE_TEST_DEBUG: "0"', 'CARGO_BUILD_JOBS: "1"',
            FULL_CHECKOUT_REF, "Record exact-ref checkout identity",
        ):
            self.assertIn(setting, applications)
            self.assertIn(setting, linked)
        self.assertNotIn("Qualify linked", applications)
        steps = job_steps(FULL, "canonical-linked-applications")
        fetch = next(step for step in steps if "Fetch locked application dependencies" in step)
        admission = next(step for step in steps if "Execute activated native host I/O and application syntax" in step)
        self.assertIn("cargo +nightly-2026-03-03 fetch --locked", fetch)
        self.assertLess(linked.index(fetch), linked.index(admission))
        upload = next(step for step in steps if "Retain linked application evidence" in step)
        self.assertIn("if: ${{ always() }}", upload)
        for evidence in (
            "canonical-native-evidence", "canonical-cli-evidence", "native-host-evidence",
            "native-control-evidence", "native-stdin-evidence",
        ):
            self.assertIn(f"${{{{ runner.temp }}}}/{evidence}", upload)
        self.assertIn("- canonical-linked-applications", job_block(FULL, "cargo"))
        self.assertNotIn("continue-on-error", linked)

    def test_canonical_application_suites_keep_all_commands_in_required_parallel_shards(self):
        block = job_block(FULL, "canonical-applications")
        self.assertIn("fail-fast: false", block)
        self.assertIn("suite: [source, native, cli]", block)
        expected = {
            "Execute canonical resident applications and retained tail controls": "source",
            "Execute canonical collection and text source contracts": "source",
            "Execute source assignment and fixed-shape specialization contracts": "source",
            "Execute canonical native lowering and selected implementation witnesses": "native",
            "Execute secured canonical reference bytecode admission": "cli",
            "Execute canonical CLI byte and resource contracts": "cli",
        }
        steps = [step for step in job_steps(FULL, "canonical-applications")
                 if re.search(r"(?m)^      - name: Execute ", step)]
        self.assertEqual(len(steps), len(expected))
        for name, suite in expected.items():
            with self.subTest(name=name):
                matching = [step for step in steps if f"- name: {name}\n" in step]
                self.assertEqual(len(matching), 1)
                self.assertIn("if: ${{ matrix.suite == '" + suite + "' }}", matching[0])
                self.assertNotIn("continue-on-error", matching[0])
                self.assertIn("--locked --offline", matching[0])
        gate = job_block(FULL, "cargo")
        self.assertIn("      - canonical-applications\n", gate)
        for family in ("language", "native"):
            self.assertRegex(gate, rf"'{family}': \[[^\n]*'canonical-applications'")

    def test_native_cli_collision_has_an_independent_required_cold_build_budget(self):
        memory = job_block(FULL, "managed-memory-runtime")
        collision = job_block(FULL, "native-cli-collision")
        self.assertEqual(int(re.search(r"timeout-minutes: (\d+)", memory).group(1)), 75)
        self.assertEqual(int(re.search(r"timeout-minutes: (\d+)", collision).group(1)), 30)
        self.assertNotIn("cranelift_kept_project_collision_precedes_emission_and_output_writes", memory)
        self.assertIn(FULL_CHECKOUT_REF, collision)
        self.assertIn("Record exact-ref checkout identity", collision)
        self.assertIn('CARGO_BUILD_JOBS: "1"', collision)
        steps = job_steps(FULL, "native-cli-collision")
        fetch = next(step for step in steps if "Fetch locked native CLI dependencies" in step)
        execute = next(step for step in steps if "Check CLI kept-project collision before native build" in step)
        self.assertIn("cargo +nightly-2026-03-03 fetch --locked", fetch)
        self.assertIn("timeout-minutes: 25", execute)
        self.assertLess(collision.index(fetch), collision.index(execute))
        commands = [" ".join(line.split()) for line in execute.replace("\\\n", " ").splitlines()]
        self.assertIn("cargo +nightly-2026-03-03 check --locked --offline -p mech --features cranelift_build", commands)
        self.assertIn("cargo +nightly-2026-03-03 test --locked --offline -p mech --features cranelift_build --lib cli::commands::build::tests::cranelift_kept_project_collision_precedes_emission_and_output_writes -- --exact --test-threads=1", commands)
        self.assertNotIn("--no-run", execute)
        self.assertNotIn("--list", execute)
        self.assertNotIn("continue-on-error", collision)
        for family in ("memory", "numeric"):
            self.assertIn(f"'{family}'", collision.splitlines()[0])
        self.assertIn("- native-cli-collision", job_block(FULL, "cargo"))

    def test_canonical_iteration_lowering_and_current_visit_proof_are_selected(self):
        steps = job_steps(FULL, "canonical-applications")
        contracts = next(step for step in steps if "Execute canonical native lowering and selected implementation witnesses" in step)
        self.assertRegex(contracts, r"--test\s+native_artifact_iteration_lowering(?:\s|$)")
        self.assertRegex(contracts, r"--test\s+canonical_artifact_iteration(?:\s|$)")
        self.assertIn("--lib resident::general::native_match_lowering::iteration_proof_tests", contracts)
        self.assertIn("--features full_compiler,native-artifact", contracts)

    def test_collection_factory_and_exact_template_contracts_are_executed(self):
        steps = job_steps(FULL, "canonical-applications")
        contracts = next(step for step in steps if "Execute canonical native lowering and selected implementation witnesses" in step)
        self.assertRegex(contracts, r"--test\s+resident_factory_lowering_attachment(?:\s|$)")
        self.assertRegex(contracts, r"--test\s+native_collection_templates(?:\s|$)")
        self.assertIn("--lib resident::general::native_collection_templates::tests", contracts)

    def test_recursive_conversion_funding_runs_in_both_source_profiles(self):
        contracts = next(step for step in job_steps(FULL, "canonical-applications")
                         if "Execute canonical native lowering and selected implementation witnesses" in step)
        # The ownership inventory deliberately admits only straight-line shell.
        # Keep both actual profiles literal so one loop cannot hide this step's
        # sixteen neighboring integration-target selections from that inventory.
        block = contracts.split("run: |\n", 1)[1].replace("\\\n", " ")
        commands = [shlex.split(line) for line in block.splitlines() if line.strip()]
        self.assertTrue(commands)
        self.assertTrue(all(command[0] == "cargo" for command in commands))
        filters = (
            "resident::set_conversion::",
            "intrinsics::kind_conversion::",
            "source_semantics::frontend::annotation_conversion_shape_tests::",
        )
        selected = []
        for command in commands:
            if "--lib" not in command or command.index("--lib") + 1 == len(command):
                continue
            selection = command[command.index("--lib") + 1]
            if selection not in filters:
                continue
            self.assertEqual(command[:3], ["cargo", "+nightly-2026-03-03", "test"])
            self.assertEqual(command[command.index("-p") + 1], "mech-engine")
            self.assertEqual(command.count("--features"), 1)
            features = command[command.index("--features") + 1]
            self.assertIn("--no-default-features", command)
            self.assertIn("--locked", command)
            self.assertIn("--offline", command)
            self.assertEqual(command[command.index("--") + 1:], ["--test-threads=1"])
            self.assertNotIn("--no-run", command)
            self.assertNotIn("--list", command)
            selected.append((features, selection))
        self.assertCountEqual(selected, [
            (f"{profile},native-artifact", selection)
            for profile in ("full_compiler", "full_source") for selection in filters
        ])
        architecture = job_block(FULL, "architecture-contracts")
        for target in ("matrix_set_declared_conversion", "conversion_staging_bounds",
                       "conversion_sort_work"):
            self.assertRegex(architecture, rf"--test\s+{target}(?:\s|$)")

    def test_shared_snapshot_ownership_regression_is_nonempty_and_executed(self):
        step = next(step for step in job_steps(FULL, "managed-memory-runtime")
                    if "Execute bounded shared snapshot ownership regression" in step)
        name = "snapshot::validation::tests::budget_registration_removal_and_same_owner_reuse_ignore_later_foreign_imports"
        self.assertEqual(step.count(name), 3)
        self.assertIn("-- --list --exact", step)
        self.assertIn("-- --exact --test-threads=1", step)
        self.assertIn(f"^{name}: test$", step)
        self.assertIn('" -eq 1', step)
        self.assertNotIn("--no-run", step)

    def test_generated_mixed_program_execution_and_runtime_only_profile_are_selected(self):
        contracts = next(step for step in job_steps(FULL, "canonical-applications")
                         if "Execute canonical native lowering and selected implementation witnesses" in step)
        commands = contracts.replace("\\\n", " ")
        self.assertRegex(commands, r"--features canonical-native-program\s+--test canonical_artifact_program\s+-- --test-threads=1")
        self.assertIn("--features canonical-native-program-runtime --lib", commands)
        self.assertRegex(commands, r"--features canonical-native-program\s+--lib native_artifact::program_runtime::tests\s+-- --test-threads=1")
        manifest = (ROOT / "hosts/gpu/Cargo.toml").read_text()
        self.assertRegex(manifest, r'name = "canonical_artifact_program"\s+required-features = \["canonical-native-program"\]')
        self.assertNotIn("--no-run", contracts)
        self.assertIn("--features functions", contracts)
        self.assertIn("--features full_compiler,native-artifact", contracts)
        self.assertNotIn("--no-run", contracts)
        linked = job_steps(FULL, "canonical-linked-applications")
        selection = next(step for step in linked if "Verify linked canonical managed program selection is nonempty" in step)
        execute = next(step for step in linked if "Qualify linked canonical managed programs" in step)
        name = "linked_qualification::linked_managed_programs_preserve_canonical_outputs_errors_and_denials"
        self.assertIn("--list --ignored --exact", selection)
        self.assertIn(name + ": test$", selection)
        self.assertIn("--features canonical-native-program", execute)
        self.assertIn("--test canonical_artifact_program", execute)
        self.assertIn("--ignored --exact", execute)
        self.assertIn(name, execute)
        self.assertIn("MECH_CANONICAL_PROGRAM_SOURCE_COMMIT", execute)
        self.assertIn("MECH_CANONICAL_PROGRAM_EVIDENCE_DIR", execute)
        upload = next(step for step in linked if "Retain linked application evidence" in step)
        self.assertIn("if: ${{ always() }}", upload)
        self.assertIn("${{ runner.temp }}/canonical-program-evidence", upload)
        reference = next(step for step in job_steps(FULL, "canonical-applications")
                         if "Execute secured canonical reference bytecode admission" in step)
        self.assertIn("--features full_compiler,resident-routing-source", reference)
        self.assertIn("--example canonical_host_reference -- --test-threads=1", reference)
        self.assertNotIn("--no-run", reference)

    def test_canonical_iteration_has_nonempty_exact_linked_qualification(self):
        steps = job_steps(FULL, "canonical-linked-applications")
        selection = next(step for step in steps if "Verify linked canonical iteration selection is nonempty" in step)
        execute = next(step for step in steps if "Qualify linked canonical iteration application" in step)
        name = "linked_qualification::one_linked_iteration_executable_preserves_exact_typed_loops_and_denials"
        self.assertIn("--list --ignored --exact", selection)
        self.assertIn(name + ": test$", selection)
        self.assertIn("--test canonical_artifact_iteration", execute)
        evidence = "${{ runner.temp }}/canonical-iteration-evidence"
        self.assertIn(f"MECH_CANONICAL_ITERATION_EVIDENCE_DIR: {evidence}", execute)
        upload = next(step for step in steps if "Retain linked application evidence" in step)
        self.assertIn("if: ${{ always() }}", upload)
        self.assertIn(evidence, upload)
        self.assertIn("--ignored --exact", execute)
        self.assertIn(name, execute)
        self.assertIn("MECH_CANONICAL_ITERATION_SOURCE_COMMIT", execute)
        self.assertIn("MECH_CANONICAL_ITERATION_EVIDENCE_DIR", execute)

    def test_canonical_match_executes_contracts_and_nonempty_linked_qualification(self):
        steps = job_steps(FULL, "canonical-applications")
        contracts = next(step for step in steps if "Execute canonical native lowering and selected implementation witnesses" in step)
        self.assertRegex(contracts, r"--test\s+native_artifact_match_lowering(?:\s|$)")
        self.assertRegex(contracts, r"--test\s+canonical_artifact_match(?:\s|$)")
        self.assertIn("--features full_compiler,native-artifact", contracts)
        self.assertIn("--features canonical-native", contracts)
        self.assertIn("--features canonical-native-runtime --lib", contracts)
        self.assertNotIn("--ignored", contracts)
        block = job_block(FULL, "canonical-linked-applications")
        steps = job_steps(FULL, "canonical-linked-applications")
        selected = "linked_canonical_match_applications_are_lazy_bounded_and_atomic"
        listing = next(step for step in steps if "Verify linked canonical Match selection is nonempty" in step)
        execution = next(step for step in steps if "Qualify linked canonical Match applications" in step)
        self.assertIn("set -euo pipefail", listing)
        self.assertIn("--list --ignored --exact", listing)
        self.assertIn(f"^{selected}: test$", listing)
        self.assertIn("-eq 1", listing)
        self.assertIn("--test canonical_artifact_match_application -- --ignored --exact", execution)
        self.assertIn(selected, execution)
        self.assertIn("--test-threads=1", execution)
        self.assertNotIn("--list", execution)
        self.assertIn("MECH_CANONICAL_MATCH_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", execution)
        self.assertIn("MECH_CANONICAL_MATCH_EVIDENCE_DIR: ${{ runner.temp }}/canonical-match-evidence", execution)
        self.assertIn("${{ runner.temp }}/canonical-match-evidence", block)
        self.assertLess(block.index(listing), block.index(execution))
        for step in (contracts, listing, execution):
            self.assertNotIn("continue-on-error", step)
            if step == contracts:
                self.assertEqual(re.findall(r"(?m)^        if: (.*)$", step),
                                 ["${{ matrix.suite == 'native' }}"])
            else:
                self.assertNotIn("if:", step)

    def test_canonical_casts_execute_both_contract_targets_and_real_linked_qualification(self):
        steps = job_steps(FULL, "canonical-applications")
        contracts = next(step for step in steps if "Execute canonical native lowering and selected implementation witnesses" in step)
        self.assertIn("--test native_artifact_cast_lowering", contracts)
        self.assertIn("--test canonical_artifact_cast", contracts)
        self.assertIn("--features full_compiler,native-artifact", contracts)
        self.assertIn("--features canonical-native", contracts)
        self.assertNotIn("--ignored", contracts)
        block = job_block(FULL, "canonical-linked-applications")
        steps = job_steps(FULL, "canonical-linked-applications")
        selected = "linked_canonical_artifact_cast_applications_preserve_checked_boundaries"
        listing = next(step for step in steps if "Verify linked canonical cast selection is nonempty" in step)
        execution = next(step for step in steps if "Qualify linked canonical cast applications" in step)
        self.assertIn("--list --ignored --exact", listing)
        self.assertIn(f"^{selected}: test$", listing)
        self.assertIn("-eq 1", listing)
        self.assertIn("--test canonical_artifact_application -- --ignored --exact", execution)
        self.assertIn(selected, execution)
        self.assertIn("--test-threads=1", execution)
        self.assertNotIn("--list", execution)
        self.assertIn("MECH_CANONICAL_NATIVE_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", execution)
        self.assertIn("MECH_CANONICAL_NATIVE_EVIDENCE_DIR: ${{ runner.temp }}/canonical-cast-evidence", execution)
        self.assertIn("${{ runner.temp }}/canonical-cast-evidence", block)
        self.assertLess(block.index(listing), block.index(execution))
        for step in (contracts, listing, execution):
            self.assertNotIn("continue-on-error", step)
            if step == contracts:
                self.assertEqual(re.findall(r"(?m)^        if: (.*)$", step),
                                 ["${{ matrix.suite == 'native' }}"])
            else:
                self.assertNotIn("if:", step)

    def test_native_linkage_workspace_lock_is_fetched_before_offline_installer_tests(self):
        block = job_block(FULL, "native-linkage-owners")
        steps = job_steps(FULL, "native-linkage-owners")
        fetch = next(step for step in steps if "Fetch locked workspace linkage dependencies" in step)
        offline = next(step for step in steps if "Execute general native-link installer contracts" in step)
        self.assertIn("cargo +nightly-2026-03-03 fetch --locked", fetch)
        self.assertNotIn("--manifest-path", fetch)
        self.assertNotIn("--offline", fetch)
        self.assertNotIn("if:", fetch)
        self.assertIn("test --locked --offline -p mech-engine", offline)
        self.assertIn("--features full_compiler,native-link", offline)
        self.assertLess(block.index(fetch), block.index("Check isolated owner native-link profiles"))
        self.assertLess(block.index(fetch), block.index(offline))
        self.assertNotIn("continue-on-error", block)

    def test_dynamic_integration_inventory_executes_in_both_contract_paths(self):
        self.assertIn("tests.ci.test_ci_test_inventory", job_block(CI, "static-architecture"))
        self.assertIn("tests/ci/test_ci_test_inventory.py", job_block(FULL, "architecture-mutations"))

    def test_inventory_yaml_dependency_is_pinned_and_installed_before_execution(self):
        requirements = (ROOT / ".github/ci/python-requirements.txt").read_text()
        self.assertIn("PyYAML==6.0.3", requirements)
        self.assertGreater(len(re.findall(r"--hash=sha256:[0-9a-f]{64}", requirements)), 1)
        for source, job, selection in (
            (CI, "static-architecture", "tests.ci.test_ci_test_inventory"),
            (FULL, "architecture-mutations", "tests/ci/test_ci_test_inventory.py"),
        ):
            block = job_block(source, job)
            install = next(step for step in job_steps(source, job) if "Install pinned CI YAML parser" in step)
            self.assertIn('python3 -m venv "$RUNNER_TEMP/ci-inventory-python"', install)
            self.assertIn("--only-binary=:all: --require-hashes -r .github/ci/python-requirements.txt", install)
            self.assertIn('>> "$GITHUB_PATH"', install)
            self.assertNotIn("continue-on-error", install)
            self.assertLess(block.index(install), block.index(selection))

    def test_source_cli_payload_parser_selection_is_nonempty_and_executed(self):
        step = next(step for step in job_steps(FULL, "cargo-runtime")
                    if "Run CLI host and context-send regression coverage" in step)
        self.assertIn("cargo +nightly-2026-03-03 test --locked -p mech-terminal --features provider --test provider --test provider_argument_count", step)
        self.assertIn("--test mech_cli_host", step)
        self.assertIn("PAYLOAD_TESTS=$(cargo +nightly-2026-03-03 test --locked --lib", step)
        self.assertIn("^cli::app::tests::payload_arguments::.*: test$", step)
        self.assertIn("-eq 6", step)
        self.assertIn("--no-default-features --features run", step)
        self.assertIn("cli::app::tests::payload_arguments -- --test-threads=1", step)
        default_test = "cli::run::tests::default_cli_payload_is_empty_and_an_explicit_factory_keeps_its_snapshot"
        self.assertIn("DEFAULT_PAYLOAD_TESTS=$(cargo +nightly-2026-03-03 test --locked --lib", step)
        self.assertIn(f"{default_test} -- --list --exact", step)
        self.assertIn(f"^{default_test}: test$", step)
        self.assertIn("-eq 1", step)
        self.assertRegex(step, re.escape(default_test) + r"\s*\\\n\s*-- --exact --test-threads=1")
        self.assertNotIn("--ignored", step)
        self.assertNotIn("continue-on-error", step)

    def test_source_cli_payload_counts_fail_closed_without_ripgrep(self):
        step = next(step for step in job_steps(FULL, "cargo-runtime")
                    if "Run CLI host and context-send regression coverage" in step)
        default_test = "cli::run::tests::default_cli_payload_is_empty_and_an_explicit_factory_keeps_its_snapshot"
        cases = (
            ("PAYLOAD_TESTS", [f"cli::app::tests::payload_arguments::case_{index}: test" for index in range(6)]),
            ("DEFAULT_PAYLOAD_TESTS", [f"{default_test}: test"]),
        )
        with tempfile.TemporaryDirectory() as directory:
            # The runner needs only the already-used POSIX awk, not ripgrep.
            awk = shutil.which("awk")
            self.assertIsNotNone(awk)
            (Path(directory) / "awk").symlink_to(awk)
            for variable, selected in cases:
                guard = next(line.strip() for line in step.splitlines()
                             if line.strip().startswith('test "$(printf')
                             and f'"${variable}"' in line)
                for listing, accepted in (
                    (selected + ["7 tests, 0 benchmarks"], True),
                    ([], False),
                    (selected[:-1], False),
                    (selected + selected[:1], False),
                    ([line.replace(": test", ": benchmark") for line in selected], False),
                    (["decoy::" + line for line in selected], False),
                ):
                    with self.subTest(variable=variable, listing=listing):
                        result = subprocess.run(
                            ["/bin/bash", "--noprofile", "--norc", "-euo", "pipefail", "-c", guard],
                            env={"PATH": directory, variable: "\n".join(listing)},
                            capture_output=True, text=True,
                        )
                        self.assertEqual(result.returncode == 0, accepted, result.stderr)
                        self.assertNotIn("command not found", result.stderr)

    def test_canonical_applications_execute_linked_qualifications_explicitly(self):
        block = job_block(FULL, "canonical-applications")
        self.assertIn("RUSTUP_TOOLCHAIN: nightly-2026-03-03", block)
        self.assertNotIn("continue-on-error", block)
        for target in (
            "applications", "tail_control", "native_artifact_lowering",
            "native_artifact_i64_lowering", "native_artifact_f64_lowering", "native_artifact_output_initializers",
            "canonical_artifact_native", "canonical_artifact_i64", "canonical_artifact_f64",
            "canonical_cli_bytes", "canonical_cli_argument_count",
        ):
            self.assertIn(f"--test {target}", block)
        self.assertIn("--lib resident::native_semantics::tests", block)
        self.assertIn("--features canonical-native --lib native_artifact::", block)
        self.assertIn("--features canonical-native-runtime --lib", block)
        cli_step = next(step for step in job_steps(FULL, "canonical-applications")
                        if "Execute canonical CLI byte and resource contracts" in step)
        self.assertIn("-p mech-build", cli_step)
        self.assertIn("--no-default-features --features standard-hosts", cli_step)
        for target in (
            "canonical_cli_bytes", "canonical_cli_argument_count", "canonical_resource_execution",
            "canonical_resource_planning", "canonical_map_features",
            "canonical_pure_text_features",
        ):
            self.assertIn(f"--test {target}", cli_step)
        self.assertIn("-- --test-threads=1", cli_step)
        self.assertNotIn("--ignored", cli_step)
        linked = job_block(FULL, "canonical-linked-applications")
        self.assertIn("RUSTUP_TOOLCHAIN: nightly-2026-03-03", linked)
        self.assertNotIn("continue-on-error", linked)
        admission_step = next(step for step in job_steps(FULL, "canonical-linked-applications")
                              if "Execute activated native host I/O and application syntax" in step)
        for target in (
            "native_host_io", "repeated_application", "native_host_application",
            "native_source_admission", "control_fsm", "control_postfix_selection",
            "control_selection", "control_signed", "idiomatic_sources",
        ):
            self.assertIn(f"--test {target}", admission_step)
        self.assertIn("--features native-build,native-io", admission_step)
        self.assertIn("-- --test-threads=1", admission_step)
        self.assertNotIn("--ignored", admission_step)
        for target, case, evidence in (
            ("native_host_application",
             "one_linked_executable_runs_secure_and_trusted_without_rebuilding",
             "MECH_NATIVE_QUALIFICATION_DIRECTORY"),
            ("canonical_artifact_application",
             "linked_canonical_artifact_scalar_applications_are_typed_bounded_and_atomic",
             "MECH_CANONICAL_NATIVE_EVIDENCE_DIR"),
            ("canonical_cli_application",
             "linked_canonical_cli_arguments_and_stdin_finish_without_drivers",
             "MECH_CANONICAL_CLI_EVIDENCE_DIR"),
            ("control_application",
             "one_linked_control_executable_preserves_signed_machine_outputs",
             "MECH_CONTROL_QUALIFICATION_DIRECTORY"),
            ("native_stdin_application",
             "one_linked_stdin_executable_preserves_unicode_snapshots_and_launch_policy",
             "MECH_NATIVE_STDIN_QUALIFICATION_DIRECTORY"),
        ):
            step = next(step for step in job_steps(FULL, "canonical-linked-applications")
                        if f"--test {target} " in step and case in step
                        and "-- --ignored --exact" in step)
            self.assertIn("cargo +nightly-2026-03-03 test --locked --offline", step)
            self.assertIn("-- --ignored --exact", step)
            self.assertIn(case, step)
            self.assertIn("--test-threads=1", step)
            self.assertIn(evidence, step)
            if target in {"control_application", "native_stdin_application", "canonical_cli_application"}:
                selection = next(selected for selected in job_steps(FULL, "canonical-linked-applications")
                                 if f"--test {target} " in selected and case in selected
                                 and "-- --list --ignored --exact" in selected)
                self.assertIn("cargo +nightly-2026-03-03 test --locked --offline", selection)
                self.assertIn(f"^{case}: test$", selection)
                self.assertIn('" -eq 1', selection)
                self.assertNotIn("continue-on-error", selection)
                self.assertNotIn("    if:", selection)
                self.assertNotIn("    if:", step)
                self.assertLess(linked.index(selection), linked.index(step))
            if target in {"control_application", "native_stdin_application"}:
                self.assertIn("--features native-build,native-io", step)
                self.assertIn("--features native-build,native-io", selection)
            if target == "control_application":
                self.assertIn('MECH_CONTROL_INLINE: "0"', step)
            if target == "canonical_cli_application":
                self.assertIn("--features standard-hosts,process-supervision", step)
                self.assertIn("--features standard-hosts,process-supervision", selection)
                self.assertIn("CLI_QUALIFICATION_TESTS", selection)
        self.assertIn("MECH_CANONICAL_NATIVE_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", linked)
        self.assertIn("MECH_CANONICAL_CLI_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", linked)
        self.assertIn("uses: actions/upload-artifact@", linked)
        for evidence in ("native-control-evidence", "native-stdin-evidence"):
            self.assertIn(f"${{{{ runner.temp }}}}/{evidence}", linked)

    def test_canonical_source_and_native_registration_targets_are_selected(self):
        groups = (
            (
                "canonical-applications", "Execute canonical collection and text source contracts",
                "mech-stdlib", "--no-default-features --features full_compiler",
                (
                    "comprehension_captures", "comprehension_work_source",
                    "control_local_geometry", "control_matrix_captures", "control_result_shapes",
                    "function_output_contexts", "knucleotide_record_source", "map_keys_comprehensions",
                    "map_source", "match_result_schemas", "matrix_block_comprehensions",
                    "postfix_selection", "primitive_snapshot_constructors", "pure_text_managed",
                    "pure_text_source", "retention_work", "sequence_sort", "set_comprehension_operands",
                    "string_generators", "string_literal_match", "string_membership_comprehensions",
                    "string_selector_shapes", "text_operations", "u64_float_conversion_source",
                    "underscore_addressing",
                ),
            ),
            (
                "cargo-language", "Execute canonical resident string and input shape contracts",
                "mech-engine", "--no-default-features --features full_compiler,resident-artifact",
                ("canonical_string_selection", "string_set_comprehension", "value_input_shapes"),
            ),
            (
                "native-linkage-owners", "Execute canonical intrinsic native-link registration",
                "mech-engine", "--no-default-features --features native-link,pure_text,map_ops,sequence_sort",
                ("map_native_linkage", "sequence_sort_native_linkage", "text_operations_native_linkage"),
            ),
            (
                "managed-memory-runtime", "Execute the complete managed-memory conformance boundary",
                "mech-core", "--all-features",
                ("managed_input_descriptor", "borrowed_string_footprint", "resident_lowering_attachment"),
            ),
        )
        for job, name, package, profile, targets in groups:
            with self.subTest(job=job, name=name):
                step = next(step for step in job_steps(FULL, job) if name in step)
                self.assertIn(f"-p {package}", step)
                self.assertIn(profile, step)
                for target in targets:
                    self.assertIn(f"--test {target}", step)
                self.assertNotIn("--ignored", step)
                self.assertNotIn("--no-run", step)
        memory = job_block(FULL, "managed-memory-runtime")
        release = next(step for step in job_steps(FULL, "managed-memory-runtime")
                       if "Execute the retained release-mode memory safety boundary" in step)
        self.assertIn('RUSTFLAGS="${RUSTFLAGS:-} -C debug-assertions=no"', release)
        self.assertIn("--release -p mech-core", release)
        self.assertIn("--features functions,u8,u64,f64,string,matrixd,bool", release)
        self.assertIn("--test memory_safety", release)
        self.assertNotIn("continue-on-error", release)
        self.assertLess(memory.index("Execute the complete managed-memory conformance boundary"), memory.index(release))
        self.assertLess(memory.index(release), memory.index("Execute maintained runtime and host memory boundaries"))

    def test_runtime_family_keeps_minimal_and_complete_source_profiles(self):
        block = job_block(FULL, "cargo-runtime")
        self.assertIn("run: cargo test -p mech-runtime --lib --tests", block)
        step = next(step for step in job_steps(FULL, "cargo-runtime")
                    if "Run complete source and compiler runtime contracts" in step)
        self.assertIn("-p mech-runtime --lib --tests", step)
        self.assertIn("'runtime'", step)
        features = set(re.search(r"--features (\S+)", step).group(1).split(","))
        self.assertTrue({"full_source", "full_compiler", "resident-routing-source",
                         "resident_external_test_support", "resident_ekf_benchmarks",
                         "runtime_bench_probes", "compute", "watcher"} <= features)

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
        self.assertIn("'numeric'", step)
        self.assertIn("'runtime'", step)
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

    def test_source_acceptance_and_shape_facts_are_selected(self):
        block = job_block(FULL, "cargo-language")
        step = next(
            step
            for step in job_steps(FULL, "cargo-language")
            if "Verify canonical document execution and rendered outputs" in step
        )
        self.assertIn("--features full_source,resident-routing-source", step)
        self.assertIn("--test source_acceptance", step)
        self.assertIn("--features full_source,resident-artifact,r64,c64", step)
        self.assertIn("--lib resident::general::shape_fact_tests::", step)
        self.assertIn(step, block)

    def test_syntax_stress_optimization_retains_assertions_and_complete_selection(self):
        step = next(
            step for step in job_steps(FULL, "cargo-language")
            if "Run syntax grammar suites" in step
        )
        for setting in (
            'CARGO_PROFILE_TEST_OPT_LEVEL: "1"',
            'CARGO_PROFILE_TEST_DEBUG_ASSERTIONS: "true"',
            'CARGO_PROFILE_TEST_OVERFLOW_CHECKS: "true"',
        ):
            self.assertIn(setting, step)
            reduced = next(selected for selected in job_steps(FULL, "cargo-language")
                           if "Run reduced syntax feature profiles" in selected)
            self.assertIn(setting, reduced)
        self.assertIn(
            "cargo +nightly-2026-03-03 test --locked -p mech-syntax --lib --tests "
            "--no-default-features --features full -- --test-threads=1", step
        )
        self.assertNotIn("--skip", step)
        self.assertNotIn("--no-run", step)
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("timeout-minutes", step)
        self._assert_retained_syntax_stress_settings(
            (ROOT / "src/syntax/tests/document_streaming_complexity.rs").read_text()
        )

    def _assert_retained_syntax_stress_settings(self, source):
        # Bind checks to executable function bodies, not comments or literals
        # that merely repeat an old bound elsewhere in the file.
        code = rust_code(source)
        self.assertNotRegex(code, r"#\s*!\s*\[", "stress target must remain unconditional")

        def enabled_test(name):
            function = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", code)
            self.assertIsNotNone(function, name)
            position = function.start()
            self.assertEqual(
                code[:position].count("{") - code[:position].count("}"), 0,
                f"{name} must remain a top-level stress entry point",
            )
            attributes = []
            while True:
                while position and code[position - 1].isspace():
                    position -= 1
                if not position or code[position - 1] != "]":
                    break
                end = position - 1
                position = end
                depth = 1
                while depth and position:
                    position -= 1
                    depth += (code[position] == "]") - (code[position] == "[")
                self.assertEqual(depth, 0, name)
                start = position
                while position and code[position - 1].isspace():
                    position -= 1
                self.assertGreater(position, 0, name)
                self.assertEqual(code[position - 1], "#", name)
                attributes.append(re.sub(r"\s+", "", code[start + 1:end]))
                position -= 1
            self.assertEqual(attributes, ["test"], f"{name} must be enabled unconditionally")

        def body(name):
            matches = list(re.finditer(rf"\bfn\s+{re.escape(name)}\s*\(", code))
            self.assertEqual(len(matches), 1, name)
            start = code.index("{", matches[0].end())
            depth = 1
            end = start + 1
            while depth and end < len(code):
                depth += (code[end] == "{") - (code[end] == "}")
                end += 1
            self.assertEqual(depth, 0, name)
            compact = re.sub(r"\s+", "", code[start + 1:end - 1])
            return re.sub(r"(?<=\d)_(?=\d)", "", compact)

        config = body("large_config")
        for field, value in {
            "max_nesting": 256,
            "max_diagnostics": 65_536,
            "max_events": 16_000_000,
            "max_recovery_bytes": 8_000_000,
            "fuel": 1_000_000_000,
            "max_source_bytes": 1_000_000,
            "max_parser_work": 2_000_000_000,
        }.items():
            self.assertIn(f"{field}:{value},", config)
        growth = body("large_growth")
        settled = body("large_settled_prefix_does_not_increase_small_append_work")
        for actual in (growth, settled):
            self.assertIn("fornin[8192,16384,32768,65536]{", actual)
        self.assertIn("workload_with_config(&source,true,clean,config,limits,64)", growth)
        self.assertIn("assert!(work.accounted<=total*3,", growth)
        self.assertIn("assert!(work.consumer.work()<=consumer*3,", growth)
        self.assertIn("assert!(costs[3]<=costs[0]*3,", settled)
        for name in (
            "large_generated_mech_consumes_every_delta",
            "large_mixed_documents_consume_every_delta",
            "large_unfinished_tokens_and_late_selection_consume_every_delta",
            "large_executable_fences_consume_every_delta",
            "large_malformed_tails_consume_every_delta",
            "large_settled_prefix_does_not_increase_small_append_work",
        ):
            enabled_test(name)
            if name != "large_settled_prefix_does_not_increase_small_append_work":
                self.assertIn("large_growth(", body(name))

    def test_syntax_stress_contract_rejects_weakened_settings_and_decoys(self):
        source = (ROOT / "src/syntax/tests/document_streaming_complexity.rs").read_text()
        for before, after in (
            ("[8_192, 16_384, 32_768, 65_536]", "[8_192, 16_384, 32_768]"),
            ("work.accounted <= total * 3", "work.accounted <= total * 4"),
            ("work.consumer.work() <= consumer * 3", "work.consumer.work() <= consumer * 4"),
            ("costs[3] <= costs[0] * 3", "costs[3] <= costs[0] * 4"),
            ("max_nesting: 256", "max_nesting: 512"),
            ("max_diagnostics: 65_536", "max_diagnostics: 131_072"),
            ("max_events: 16_000_000", "max_events: 32_000_000"),
            ("max_recovery_bytes: 8_000_000", "max_recovery_bytes: 16_000_000"),
            ("fuel: 1_000_000_000", "fuel: 1"),
            ("max_source_bytes: 1_000_000", "max_source_bytes: 2_000_000"),
            ("max_parser_work: 2_000_000_000", "max_parser_work: 4_000_000_000"),
            ("config, limits, 64)", "config, limits, 128)"),
            ("#[test]\nfn large_generated_mech", "#[ignore]\nfn large_generated_mech"),
        ):
            with self.subTest(setting=before):
                mutated = source.replace(before, after)
                self.assertNotEqual(mutated, source)
                # Neither a comment nor a string containing the retained
                # spelling can rescue a weakened executable setting.
                mutated += (
                    "\n// " + before.replace("\n", "\n// ")
                    + "\nconst DECOY: &str = " + json.dumps(before) + ";\n"
                )
                with self.assertRaises(AssertionError):
                    self._assert_retained_syntax_stress_settings(mutated)

    def test_syntax_stress_contract_rejects_disabling_attributes_in_any_order(self):
        source = (ROOT / "src/syntax/tests/document_streaming_complexity.rs").read_text()
        for name in (
            "large_generated_mech_consumes_every_delta",
            "large_mixed_documents_consume_every_delta",
            "large_unfinished_tokens_and_late_selection_consume_every_delta",
            "large_executable_fences_consume_every_delta",
            "large_malformed_tails_consume_every_delta",
            "large_settled_prefix_does_not_increase_small_append_work",
        ):
            original = f"#[test]\nfn {name}"
            for disabled in (
                "#[ignore]", '#[ignore = "slow"]', "#[cfg(any())]",
                "#[cfg_attr(all(), ignore)]", "#[cfg_attr(all(), cfg(any()))]",
            ):
                for attributes in (f"{disabled}\n#[test]", f"#[test]\n{disabled}"):
                    with self.subTest(name=name, attributes=attributes):
                        mutated = source.replace(original, f"{attributes}\nfn {name}", 1)
                        self.assertNotEqual(mutated, source)
                        with self.assertRaises(AssertionError):
                            self._assert_retained_syntax_stress_settings(mutated)
        for disabled in (
            "#![cfg(any())]\n" + source,
            "#[cfg(any())]\nmod hidden {\n" + source + "\n}\n",
        ):
            with self.assertRaises(AssertionError):
                self._assert_retained_syntax_stress_settings(disabled)

    def test_full_source_profile_is_bounded_and_independent(self):
        profile = job_block(CI, "full-source-profile")
        self.assertIn("standard_canaries_required == 'true'", profile)
        self.assertIn("--timeout-secs 540", profile)
        manifest = "tests/fixtures/full-source-runtime/Cargo.toml"
        self.assertIn(f"fetch --locked --manifest-path {manifest}", profile)
        self.assertIn(f"run --locked --offline --manifest-path {manifest}", profile)
        self.assertIn("- full-source-profile", job_block(CI, "pr-gate"))
        self.assertNotIn(manifest, job_block(CI, "standard-linux"))
        browser = job_block(CI, "browser-compute-smoke")
        self.assertNotIn(manifest, browser)
        self.assertIn("smoke-canonical-document-bundle-browser.py --served-compute", browser)

    def test_source_fixture_default_profile_has_no_large_debug_or_incremental_build(self):
        import tomllib

        source = (ROOT / "tests/fixtures/full-source-runtime/Cargo.toml").read_text()
        profile = re.search(r"(?ms)^\[profile\.dev\]\n.*?(?=^\[|\Z)", source)
        self.assertIsNotNone(profile, "the isolated fixture must own its build profile")
        manifest = tomllib.loads(profile.group())
        self.assertEqual(manifest["profile"]["dev"]["debug"], 0)
        self.assertIs(manifest["profile"]["dev"]["incremental"], False)

    def test_shared_browser_product_checks_revision_profile_lock_and_contents(self):
        producer = job_block(CI, "browser-compute-build")
        self.assertIn("sha256sum target/debug/mech", producer)
        self.assertIn("find src/wasm/pkg -type f -print0", producer)
        for consumer in ("browser-compute-smoke", "browser-ekf-cpu", "browser-ekf-wgpu"):
            with self.subTest(consumer=consumer):
                step = next(step for step in job_steps(CI, consumer)
                            if "Restore the shared compute browser build" in step)
                for identity in ("revision", "tree", "profile", "lock.sha256", "products.sha256"):
                    self.assertIn(f"target/browser-compute-provenance/{identity}", step)
                self.assertIn('= "$GITHUB_SHA"', step)
                self.assertIn("HEAD^{tree}", step)
                self.assertNotIn("continue-on-error", step)

    def test_shared_browser_product_rejects_changed_or_missing_provenance(self):
        step = next(step for step in job_steps(CI, "browser-compute-smoke")
                    if "Restore the shared compute browser build" in step)
        guards = textwrap.dedent(step.split("        run: |\n", 1)[1]).splitlines()[1:]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "Cargo.lock").write_text("locked graph\n")
            subprocess.run(["git", "add", "Cargo.lock"], cwd=root, check=True)
            commit_env = os.environ | {
                "GIT_AUTHOR_NAME": "CI contract", "GIT_AUTHOR_EMAIL": "ci@example.invalid",
                "GIT_COMMITTER_NAME": "CI contract", "GIT_COMMITTER_EMAIL": "ci@example.invalid",
            }
            subprocess.run(["git", "commit", "-qm", "fixture"], cwd=root, env=commit_env, check=True)
            sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            tree = subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=root, text=True).strip()
            provenance = root / "target/browser-compute-provenance"
            provenance.mkdir(parents=True)
            binary = root / "target/debug/mech"
            binary.parent.mkdir()
            binary.write_bytes(b"product")
            values = {
                "revision": sha,
                "tree": tree,
                "profile": "mech:dev:compute_backends_native;wasm:browser-compute-canary:wasm32-unknown-unknown",
                "lock.sha256": hashlib.sha256((root / "Cargo.lock").read_bytes()).hexdigest() + "  Cargo.lock",
                "products.sha256": hashlib.sha256(binary.read_bytes()).hexdigest() + "  target/debug/mech",
            }
            for name, value in values.items():
                (provenance / name).write_text(value + "\n")
            def accepts():
                return subprocess.run(["bash", "-e", "-c", "\n".join(guards)], cwd=root,
                                      env=os.environ | {"GITHUB_SHA": sha},
                                      capture_output=True).returncode == 0
            self.assertTrue(accepts())
            for name, value in values.items():
                with self.subTest(missing=name):
                    (provenance / name).unlink()
                    self.assertFalse(accepts())
                    (provenance / name).write_text(value + "\n")
            for path in (root / "Cargo.lock", binary, provenance / "revision",
                         provenance / "tree", provenance / "profile"):
                with self.subTest(changed=path.name):
                    original = path.read_bytes()
                    path.write_bytes(b"changed")
                    self.assertFalse(accepts())
                    path.write_bytes(original)

    def test_browser_aggregate_requires_exactly_the_selected_application_cases(self):
        block = job_block(CI, "browser-compute-canary")
        script = textwrap.dedent(block.split("        run: |\n", 1)[1])
        def accepts(browser, applications, **overrides):
            environment = {
                "REQUIRED": str(browser).lower(),
                "APPLICATIONS_REQUIRED": str(applications).lower(),
                "BUILD_RESULT": "success" if browser else "skipped",
                "SMOKE_RESULT": "success" if browser else "skipped",
                **{f"{kind}_RESULT": "success" if applications else "skipped"
                   for kind in ("CPU", "WGPU", "PARITY")},
                **overrides,
            }
            return subprocess.run(["bash", "-e", "-c", script], env=environment,
                                  capture_output=True).returncode == 0
        for browser, applications in ((False, False), (True, False), (True, True)):
            self.assertTrue(accepts(browser, applications))
        for result in ("failure", "cancelled", "skipped", ""):
            self.assertFalse(accepts(True, True, CPU_RESULT=result))
            self.assertFalse(accepts(True, False, BUILD_RESULT=result))
        self.assertFalse(accepts(True, False, APPLICATIONS_REQUIRED=""))

    def test_native_plan_starts_early_once_on_the_integration_revision(self):
        early = job_block(CI, "early-native-plan")
        delegated = job_block(FULL, "native-plan")
        caller = job_block(CI, "full-validation")
        self.assertIn("needs: impact", early)
        self.assertNotIn("needs:\n", early)
        self.assertIn("if: needs.impact.outputs.native_validation_required == 'true'", early)
        self.assertIn("validation_ref: ${{ github.sha }}", early)
        self.assertIn("native_plan_in_caller: true", caller)
        for block in (early, delegated):
            self.assertIn("uses: ./.github/workflows/ci-native-plan.yml", block)
            self.assertNotIn("steps:", block)
            self.assertNotIn("continue-on-error", block)
        self.assertIn("&& !inputs.native_plan_in_caller", delegated)
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
        planning = "--stage planning-execution"
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

    def test_secured_canonical_hosted_target_is_required_by_the_existing_native_gate(self):
        native = job_block(NATIVE, "canonical-hosted")
        settings = native.split("    steps:\n", 1)[0]
        self.assertNotRegex(settings, r"(?m)^    if:")
        self.assertNotIn("continue-on-error", native)
        self.assertNotIn("--skip", native)
        self.assertRegex(settings, r"(?m)^    timeout-minutes: [1-9][0-9]*$")
        self.assertIn("RUSTUP_TOOLCHAIN: nightly-2026-03-03", settings)
        self.assertIn('CARGO_BUILD_JOBS: "1"', settings)
        self.assertIn('CARGO_INCREMENTAL: 0', settings)
        self.assertIn("ref: ${{ inputs.validation_ref }}", native)
        self.assertIn('test "$actual" = "$expected"', native)
        self.assertIn('test "$actual" = "$CHECKOUT_SHA"', native)
        # Both callers already aggregate the ENTIRE reusable workflow, not
        # one child job. A failed new child therefore fails the existing gate.
        for source, job in ((CI, "early-native-plan"), (FULL, "native-plan")):
            self.assertIn("uses: ./.github/workflows/ci-native-plan.yml", job_block(source, job))
        self.assertIn("- early-native-plan", job_block(CI, "pr-gate"))
        self.assertIn("- native-plan", job_block(FULL, "cargo"))
        steps = job_steps(NATIVE, "canonical-hosted")
        def named(name):
            return next(step for step in steps if f"- name: {name}\n" in step)
        compile_step = named("Compile canonical hosted contracts without execution")
        execute = named("Execute complete canonical hosted contracts")
        select = named("Verify linked canonical hosted selection is nonempty")
        qualify = named("Qualify original and decoded secured hosted standalone applications")
        for selected in (compile_step, execute, select, qualify):
            self.assertNotRegex(selected, r"(?m)^        if:")
        self.assertIn("--record-test-executable canonical_hosted_artifact", compile_step)
        self.assertIn("--test canonical_hosted_artifact --no-run --message-format=json-render-diagnostics", compile_step)
        self.assertEqual(
            re.findall(r"(?:^|\s)-p\s+(\S+)", compile_step.replace("\\\n", " ")),
            ["mech-core", "mech-engine", "mech-runtime", "mech-stdlib", "mech-syntax", "mech-gpu"],
        )
        self.assertIn("--no-default-features", compile_step)
        self.assertIn("mech-stdlib/full_compiler,mech-stdlib/native-artifact,mech-gpu/canonical-native-program-host", compile_step)
        for selected in (execute, select, qualify):
            self.assertIn("--test-executable-file target/native-plan-logs/canonical-hosted-executable", selected)
            self.assertNotIn("cargo +", selected)
        self.assertIn("--test-threads=1 --nocapture", execute)
        self.assertNotIn("--ignored", execute)
        self.assertNotIn("--exact", execute)
        self.assertNotIn("--list", execute)
        case = "linked::complete_original_and_decoded_knucleotide_linked_processes_are_secured_and_pinned"
        self.assertIn("--list --ignored --exact", select)
        self.assertIn(case, select)
        self.assertIn(f'expected = "{case}: test"; assert lines.count(expected) == 1', select)
        self.assertIn("--ignored --exact", qualify)
        self.assertIn(case, qualify)
        self.assertIn("--test-threads=1 --nocapture", qualify)
        self.assertIn("MECH_CANONICAL_HOSTED_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", qualify)
        self.assertIn("MECH_CANONICAL_HOSTED_EVIDENCE_DIR: ${{ runner.temp }}/canonical-hosted-evidence", qualify)
        self.assertNotIn("mkdir", qualify)  # the qualifier requires a fresh nonexistent child
        self.assertNotIn("CARGO_HOME", native)  # preserve ambient-config refusal, not a hidden override
        self.assertLess(steps.index(compile_step), steps.index(execute))
        self.assertLess(steps.index(execute), steps.index(select))
        self.assertLess(steps.index(select), steps.index(qualify))
        # Two variants each run twelve input cases plus closed stdout. Include
        # the unused-argv acquisition/memory denials without shortening any
        # existing child bound to fit the enclosing supervisor.
        inner_ceilings = 2 * 900 + 2 * 120 + 2 * 300 + 26 * 120 + 10 * 120 + 4 * 15
        self.assertGreaterEqual(
            int(re.search(r"--timeout-secs (\d+)", qualify).group(1)),
            inner_ceilings + 300,
        )
        for selected in (compile_step, execute, select, qualify):
            timeout = int(re.search(r"--timeout-secs (\d+)", selected).group(1))
            actions = int(re.search(r"timeout-minutes: (\d+)", selected).group(1)) * 60
            self.assertGreater(actions, timeout + 30 + 5 + 15)
        retained = named("Retain canonical hosted diagnostics and qualification evidence")
        self.assertIn("if: always()", retained)
        self.assertIn("target/native-plan-logs", retained)
        self.assertIn("${{ runner.temp }}/canonical-hosted-evidence", retained)
        self.assertIn("retention-days: 14", retained)
        fixture = (ROOT / "hosts/gpu/tests/support/canonical_hosted_linked.rs").read_text(encoding="utf-8")
        self.assertIn("fn " + case.split("::")[-1] + "()", fixture)
        self.assertIn("MECH_CANONICAL_HOSTED_SOURCE_COMMIT", fixture)
        self.assertIn("MECH_CANONICAL_HOSTED_EVIDENCE_DIR", fixture)
        self.assertIn("template_tests::qualify(", fixture)
        generated = (ROOT / "hosts/gpu/tests/support/canonical_hosted_template_tests.rs").read_text(encoding="utf-8")
        self.assertIn("CargoInvocation::test_binary(", generated)
        self.assertIn('"generated-template-tests", 300', generated)
        self.assertIn("suite_passed(&output.stdout)", generated)
        for label in (
            "unused-arguments-exact-cap",
            "unused-arguments-over-cap",
            "argument-header-memory-denial",
        ):
            self.assertIn(f'("{label}", GGTA_INPUT,', fixture)


    def test_same_binary_policy_job_is_required_explicit_nonempty_and_bounded(self):
        native = job_block(NATIVE, "canonical-policy")
        settings = native.split("    steps:\n", 1)[0]
        self.assertNotRegex(settings, r"(?m)^    if:")
        for forbidden in ("continue-on-error", "--skip", "CARGO_HOME"):
            self.assertNotIn(forbidden, native)
        self.assertIn("ref: ${{ inputs.validation_ref }}", native)
        self.assertIn('test "$actual" = "$expected"', native)
        self.assertIn('test "$actual" = "$CHECKOUT_SHA"', native)
        steps = job_steps(NATIVE, "canonical-policy")
        def named(name):
            return next(step for step in steps if f"- name: {name}\n" in step)
        compile_step = named("Compile benchmark policy contracts without execution")
        self.assertIn("mech-gpu/canonical-native-benchmark-policy", compile_step)
        self.assertIn("--record-test-executable canonical_hosted_artifact", compile_step)
        self.assertIn("--no-run --message-format=json-render-diagnostics", compile_step)
        kernel = named("Execute benchmark policy kernel contracts")
        self.assertIn("--lib native_artifact::benchmark_policy::", kernel)
        self.assertIn('[("13", "0", "0")]', kernel)
        case = "linked::benchmark::complete_original_and_decoded_knucleotide_same_binary_policy_controls"
        select = named("Verify linked benchmark policy selection is nonempty")
        qualify = named("Qualify same-binary policies and secured production controls")
        for step in (select, qualify):
            self.assertIn("--test-executable-file target/native-plan-logs/canonical-policy-executable", step)
            self.assertIn(case, step)
            self.assertNotIn("cargo +", step)
            self.assertNotRegex(step, r"(?m)^        if:")
        self.assertIn("--list --ignored --exact", select)
        self.assertIn(f'expected = "{case}: test"; assert lines.count(expected) == 1', select)
        self.assertIn("--ignored --exact", qualify)
        self.assertIn("MECH_CANONICAL_POLICY_SOURCE_COMMIT: ${{ steps.checkout.outputs.commit }}", qualify)
        self.assertIn("MECH_CANONICAL_POLICY_EVIDENCE_DIR: ${{ runner.temp }}/canonical-policy-evidence", qualify)
        self.assertIn("MECH_CANONICAL_HOSTED_PROFILE: release", qualify)
        self.assertGreaterEqual(int(re.search(r"--timeout-secs (\d+)", qualify).group(1)),
                                3 * 900 + 3 * 120 + 2 * 300 + 55 * 120 + 10 * 120 + 4 * 15 + 300)
        step_limits = []
        for step in steps:
            limit = re.search(r"(?m)^        timeout-minutes: ([1-9][0-9]*)$", step)
            self.assertIsNotNone(limit, step)
            step_limits.append(int(limit.group(1)))
            if "python3 scripts/run-native-plan-stage.py" in step:
                timeout = int(re.search(r"--timeout-secs (\d+)", step).group(1))
                self.assertGreater(int(limit.group(1)) * 60, timeout + 30 + 5 + 15)
        self.assertGreaterEqual(int(re.search(r"timeout-minutes: (\d+)", settings).group(1)), sum(step_limits) + 10)
        upload = named("Retain benchmark policy diagnostics and qualification evidence")
        for marker in ("if: always()", "target/native-plan-logs", "${{ runner.temp }}/canonical-policy-evidence", "if-no-files-found: error"):
            self.assertIn(marker, upload)
        for source, job in ((CI, "early-native-plan"), (FULL, "native-plan")):
            self.assertIn("uses: ./.github/workflows/ci-native-plan.yml", job_block(source, job))
        self.assertIn("- early-native-plan", job_block(CI, "pr-gate"))
        self.assertIn("- native-plan", job_block(FULL, "cargo"))
        production = job_block(NATIVE, "canonical-native-cli")
        self.assertNotIn("canonical-native-benchmark-policy", production)

    def test_public_canonical_native_cli_is_required_and_runs_saved_exact_selections(self):
        native = job_block(NATIVE, "canonical-native-cli")
        settings = native.split("    steps:\n", 1)[0]
        self.assertNotRegex(settings, r"(?m)^    if:")
        for forbidden in ("continue-on-error", "--skip", "--no-default-features", "CARGO_HOME"):
            self.assertNotIn(forbidden, native)
        self.assertIn("RUSTUP_TOOLCHAIN: nightly-2026-03-03", settings)
        self.assertIn('CARGO_BUILD_JOBS: "1"', settings)
        self.assertIn("ref: ${{ inputs.validation_ref }}", native)
        self.assertIn('test "$actual" = "$expected"', native)
        self.assertIn('test "$actual" = "$CHECKOUT_SHA"', native)
        for source, job in ((CI, "early-native-plan"), (FULL, "native-plan")):
            self.assertIn("uses: ./.github/workflows/ci-native-plan.yml", job_block(source, job))
        self.assertIn("- early-native-plan", job_block(CI, "pr-gate"))
        self.assertIn("- native-plan", job_block(FULL, "cargo"))
        self.assertIn("native-cli-collision", FULL)  # existing collision coverage is additive
        steps = job_steps(NATIVE, "canonical-native-cli")
        def named(name):
            return next(step for step in steps if f"- name: {name}\n" in step)
        lib = named("Compile public canonical CLI unit contracts without execution")
        public = named("Compile public canonical CLI process contracts without execution")
        for step in (lib, public):
            self.assertIn("cargo +nightly-2026-03-03 test --locked --offline -j1 -p mech", step)
            self.assertIn("--features full_compiler,cranelift_build", step)
            self.assertIn("--no-run --message-format=json-render-diagnostics", step)
        self.assertIn("--record-test-executable mech", lib)
        self.assertIn("--lib", lib)
        self.assertIn("--record-test-executable canonical_native_build", public)
        self.assertIn("--test canonical_native_build", public)
        units = named("Execute public canonical CLI unit contracts")
        qualify = named("Qualify all public canonical CLI secured process cases")
        unit_selection = named("Verify public canonical CLI unit selection is exact")
        public_selection = named("Verify public canonical CLI process selection is exact")
        for step in (unit_selection, units):
            self.assertIn("--test-executable-file target/native-plan-logs/canonical-cli-lib-executable", step)
            self.assertIn("cli::commands::build::cranelift::tests::", step)
        for step in (public_selection, qualify):
            self.assertIn("--test-executable-file target/native-plan-logs/canonical-cli-public-executable", step)
        self.assertIn("--list cli::commands::build::cranelift::tests::", unit_selection)
        self.assertIn("check_canonical_cli_ci.py unit-selection", unit_selection)
        self.assertIn("--list --ignored", public_selection)
        self.assertIn("check_canonical_cli_ci.py public-selection", public_selection)
        self.assertIn("--ignored --test-threads=1 --nocapture", qualify)
        self.assertNotIn("--exact", qualify)
        for step in (unit_selection, units, public_selection, qualify):
            self.assertNotIn("cargo +", step)
            self.assertNotRegex(step, r"(?m)^        if:")
        self.assertIn("--test-threads=1 --nocapture", units)
        self.assertNotIn("--ignored", units)
        ordered = (lib, public, named("Pin exact saved public CLI executables"),
                   unit_selection, units, public_selection, qualify,
                   named("Verify public canonical CLI completion and immutable pins"))
        self.assertEqual([steps.index(step) for step in ordered],
                         sorted(steps.index(step) for step in ordered))
        self.assertEqual(native.count("python3 scripts/run-native-plan-stage.py"), 6)

    def test_public_canonical_cli_evidence_checks_counts_pins_and_all_process_outcomes(self):
        native = job_block(NATIVE, "canonical-native-cli")
        checker = (ROOT / "scripts/check_canonical_cli_ci.py").read_text(encoding="utf-8")
        self.assertIn("len(names) == len(expected) and set(names) == expected", checker)
        self.assertIn('passed_count(log_dir / "canonical-cli-unit-contracts.log", 5)', checker)
        self.assertIn('passed_count(log_dir / "canonical-cli-public-qualification.log", 3)', checker)
        for required in ('"owned_group_cleanup") == "complete"', '"command_exit_status") == 0',
                         '"reason") == "exit"', 'target.get("kind") == ["bin"]',
                         'message.get("reason") == "compiler-artifact"',
                         'digest(path) == entry["sha256"]', "verify_pins(log_dir)",
                         "len(directories) == 3", "len(receipts) == len(expected)",
                         'receipt.get("state") == "completed"', 'receipt.get("success") is success',
                         'receipt[f"{stream}_sha256"]'):
            self.assertIn(required, checker)
        self.assertIn('scripts/tests/test_canonical_cli_ci.py', native)
        self.assertIn("MECH_CANONICAL_CLI_EVIDENCE_DIR: ${{ runner.temp }}/canonical-cli-evidence", native)
        self.assertIn('test ! -e "$RUNNER_TEMP/canonical-cli-evidence"', native)
        self.assertIn("sha256sum Cargo.toml Cargo.lock", native)
        self.assertIn("sha256sum --check target/native-plan-logs/canonical-cli-source.sha256", native)
        self.assertIn('test "$(git rev-parse HEAD)" = "$CHECKOUT_SHA"', native)
        for path in ("src/cli/commands/build_cranelift.rs",
                     "hosts/gpu/Cargo.toml",
                     "src/cli/commands/build_cranelift_tests.rs", "tests/canonical_native_build.rs",
                     "tests/support/canonical_native_build.rs",
                     "hosts/gpu/tests/fixtures/native/knucleotide-idiomatic.mec",
                     "src/runtime/src/runtime/program/compiler_retained_input.rs",
                     "src/build/src/cargo/command.rs"):
            self.assertIn(path, native)
        steps = job_steps(NATIVE, "canonical-native-cli")
        verify = next(step for step in steps if "- name: Verify public canonical CLI completion and immutable pins\n" in step)
        upload = next(step for step in steps if "- name: Retain public canonical CLI diagnostics and process evidence\n" in step)
        self.assertIn("if: always()", verify)
        self.assertIn("check_canonical_cli_ci.py complete", verify)
        for text in ("if: always()", "target/native-plan-logs", "${{ runner.temp }}/canonical-cli-evidence",
                     "if-no-files-found: error", "retention-days: 14"):
            self.assertIn(text, upload)

    def test_public_canonical_cli_deadlines_cover_inner_cases_and_every_serial_step(self):
        native = job_block(NATIVE, "canonical-native-cli")
        settings = native.split("    steps:\n", 1)[0]
        step_limits = []
        for step in job_steps(NATIVE, "canonical-native-cli"):
            limit = re.search(r"(?m)^        timeout-minutes: ([1-9][0-9]*)$", step)
            self.assertIsNotNone(limit, step)
            step_limits.append(int(limit.group(1)))
            if "python3 scripts/run-native-plan-stage.py" in step:
                timeout = int(re.search(r"--timeout-secs (\d+)", step).group(1))
                self.assertGreater(int(limit.group(1)) * 60, timeout + 30 + 5 + 15)
        job_limit = int(re.search(r"(?m)^    timeout-minutes: ([1-9][0-9]*)$", settings).group(1))
        self.assertGreaterEqual(job_limit, sum(step_limits) + 10)
        qualify = next(step for step in job_steps(NATIVE, "canonical-native-cli")
                       if "- name: Qualify all public canonical CLI secured process cases\n" in step)
        inner = (120 + 2 * 1800 + 4 * 120) + (1800 + 2 * 120) + (1800 + 120)
        self.assertGreaterEqual(int(re.search(r"--timeout-secs (\d+)", qualify).group(1)), inner + 300)

    def test_hosted_job_deadline_covers_every_serial_step_and_cleanup(self):
        settings = job_block(NATIVE, "canonical-hosted").split("    steps:\n", 1)[0]
        job_limit = re.search(r"(?m)^    timeout-minutes: ([1-9][0-9]*)$", settings)
        self.assertIsNotNone(job_limit)
        step_limits = []
        for step in job_steps(NATIVE, "canonical-hosted"):
            # Checkout and always-run upload also need explicit finite bounds.
            limit = re.search(r"(?m)^        timeout-minutes: ([1-9][0-9]*)$", step)
            self.assertIsNotNone(limit, step)
            step_limits.append(int(limit.group(1)))
        self.assertGreaterEqual(int(job_limit.group(1)), sum(step_limits) + 10)

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
        retained = [step for step in steps if "uses: actions/upload-artifact@" in step]
        self.assertEqual(len(retained), 1)
        self.assertIn("if: always()", retained[0])
        self.assertIn("target/native-plan-logs", retained[0])
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
        # Five execution commands plus the separately bounded macOS cold setup.
        self.assertEqual(len(generated_commands), 6)
        macos = job_block(FULL, "native-macos")
        self.assertEqual(macos.count("--ignored --exact prepare_generated_owner_runners"), 1)
        self.assertIn('MECH_NATIVE_OWNER_REQUIRE_PREBUILT: "1"', macos)
        self.assertLess(
            macos.index("prepare_generated_owner_runners"),
            macos.index("- name: Build selected native fixture families"),
        )

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
        self.assertIn("--stage planning-execution", native)

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

    def test_native_hosted_requires_admission_and_explicit_linked_launch_qualification(self):
        block = job_block(FULL, "native-hosted")
        self.assertIn("RUSTUP_TOOLCHAIN: nightly-2026-03-03", block)
        self.assertIn("CARGO_BUILD_JOBS: \"1\"", block)
        self.assertNotIn("continue-on-error", block)
        steps = job_steps(FULL, "native-hosted")
        admission = next(step for step in steps
                         if "Execute activated native admission and pipe regressions" in step)
        linked = next(step for step in steps
                      if "Qualify activated native launches with one linked executable" in step)
        for step in (admission, linked):
            self.assertIn("cargo +nightly-2026-03-03 test --locked --offline", step)
            self.assertIn("--features native-build,native-io", step)
            self.assertIn("--test native_host_application", step)
            self.assertIn("--test-threads=1", step)
        for target in ("native_host_io", "repeated_application", "native_source_admission"):
            self.assertIn(f"--test {target}", admission)
        self.assertNotIn("--ignored", admission)
        self.assertIn("-- --ignored --exact", linked)
        self.assertIn("one_linked_executable_runs_secure_and_trusted_without_rebuilding", linked)
        self.assertIn("MECH_NATIVE_QUALIFICATION_DIRECTORY", linked)
        self.assertIn("uses: actions/upload-artifact@", block)
        self.assertIn("- native-hosted", job_block(FULL, "cargo"))

    def test_native_commands_preserve_bounded_execution_and_final_diagnostics(self):
        for target in ("native_literal", "native_scalar", "native_generated_end_to_end", "native_cli_hosted", "mech_build"):
            self.assertIn(f"--test {target}", NATIVE_APPLICATIONS)
        self.assertNotIn("--skip", NATIVE_APPLICATIONS)
        self.assertNotIn("continue-on-error", NATIVE_APPLICATIONS)
        self.assertIn("uses: actions/upload-artifact@v4", NATIVE_APPLICATIONS)
        self.assertIn("target/native-plan-logs", NATIVE_APPLICATIONS)
        self.assertIn("if: always()", NATIVE_APPLICATIONS)
        self.assertIn("python3 scripts/probe-unix-kill-targeting.py", NATIVE)
        self.assertIn("python3 -B scripts/tests/test_probe_unix_kill_targeting.py", NATIVE)
        reuse = next(step for step in job_steps(NATIVE, "native-plan")
                     if "- name: Verify fresh-process owner reuse and planning execution" in step)
        self.assertIn("if: inputs.verify_reuse", reuse)
        self.assertIn('MECH_NATIVE_OWNER_REQUIRE_PREBUILT: "1"', reuse)

    def test_native_plan_handoff_gates_fail_closed(self):
        def accepts(block, **overrides):
            script = textwrap.dedent(block.split("        run: |\n", 1)[1])
            environment = dict.fromkeys(re.findall(r"^          ([A-Z0-9_]+):", block, re.M), "success")
            environment["SOURCE_REQUIRED"] = "true"
            environment["WINDOWS_REQUIRED"] = "true"
            environment["NATIVE_REQUIRED"] = "true"
            environment["OWNERS_REQUIRED"] = "true"
            environment.update(overrides)
            return subprocess.run(
                ["/bin/bash", "-e", "-c", script], env=environment,
                capture_output=True, text=True,
            ).returncode == 0

        pr = job_block(CI, "pr-gate")
        self.assertTrue(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true"))
        for result in ("failure", "cancelled", "skipped", ""):
            with self.subTest(result=result):
                self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", NATIVE_PLAN_RESULT=result))
                self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", SOURCE_RESULT=result))
                self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", OWNER_RESULT=result))
        self.assertTrue(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="false", NATIVE_REQUIRED="false", FULL_RESULT="skipped", NATIVE_PLAN_RESULT="skipped"))
        self.assertTrue(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="false", NATIVE_REQUIRED="false", FULL_RESULT="skipped", NATIVE_PLAN_RESULT="skipped", SOURCE_REQUIRED="false", SOURCE_RESULT="skipped"))
        self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", SOURCE_REQUIRED="unknown"))
        self.assertTrue(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", NATIVE_REQUIRED="false", NATIVE_PLAN_RESULT="skipped"))
        self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="unknown"))
        self.assertFalse(accepts(pr, DOCS_ONLY="false", CONTRACTS_REQUIRED="true", NATIVE_REQUIRED="unknown"))

    def test_docs_only_gate_accepts_successful_browser_skip_verification(self):
        block = job_block(CI, "pr-gate")
        script = textwrap.dedent(block.split("        run: |\n", 1)[1])
        environment = dict.fromkeys(re.findall(r"^          ([A-Z0-9_]+):", block, re.M), "skipped")
        environment.update(DOCS_ONLY="true", CONTRACTS_REQUIRED="false", NATIVE_REQUIRED="false", OWNERS_REQUIRED="false", IMPACT_RESULT="success", BROWSER_RESULT="success", SOURCE_REQUIRED="false", WINDOWS_REQUIRED="false")
        def accepts(**changes):
            return subprocess.run(["/bin/bash", "-e", "-c", script],
                env=environment | changes, capture_output=True).returncode == 0
        self.assertTrue(accepts())
        for result in ("failure", "cancelled", "skipped", ""):
            with self.subTest(result=result):
                self.assertFalse(accepts(BROWSER_RESULT=result))
        self.assertFalse(accepts(STATIC_RESULT="failure"))
        self.assertFalse(accepts(CONTRACTS_REQUIRED="true"))
        self.assertTrue(accepts(CONTRACTS_REQUIRED="true", NATIVE_REQUIRED="true", FULL_RESULT="success", NATIVE_PLAN_RESULT="success"))

    def test_selected_contract_gate_rejects_missing_failed_and_unselected_work(self):
        block = job_block(FULL, "cargo")
        shell = textwrap.dedent(block.split("        run: |\n", 1)[1])
        python = shell.split("python3 - <<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]
        groups = literal_assignment(python, "groups")
        dependencies = set(re.findall(r"^      - ([a-z0-9-]+)$", block, re.M))
        self.assertEqual(dependencies, {job for jobs in groups.values() for job in jobs})
        all_jobs = set(re.findall(r"^  ([a-z0-9-]+):$", FULL.split("jobs:\n", 1)[1], re.M))
        self.assertEqual(dependencies, all_jobs - {"cargo"})
        for job in dependencies:
            expected_families = {family for family, jobs in groups.items() if job in jobs}
            condition = job_block(FULL, job).splitlines()[0]
            self.assertEqual(set(re.findall(r"'([a-z]+)'", condition)), expected_families | {"all"})

        def accepts(selected, in_caller=False, changes=None, omitted=None):
            expected = {job for family, jobs in groups.items()
                        if "all" in selected or family in selected for job in jobs}
            if in_caller:
                expected.discard("native-plan")
            results = {job: {"result": "success" if job in expected else "skipped"}
                       for job in dependencies}
            for job, result in (changes or {}).items():
                results[job] = {"result": result}
            if omitted:
                results.pop(omitted)
            environment = os.environ | {
                "CONTRACT_FAMILIES": json.dumps(selected),
                "CONTRACT_RESULTS": json.dumps(results),
                "NATIVE_PLAN_IN_CALLER": str(in_caller).lower(),
            }
            return subprocess.run(["/bin/bash", "-e", "-c", shell], env=environment,
                                  capture_output=True).returncode == 0

        for family in ["all", *groups]:
            with self.subTest(family=family):
                self.assertTrue(accepts([family]))
                selected_job = next(iter(dependencies)) if family == "all" else groups[family][0]
                for result in ("failure", "cancelled", "skipped", ""):
                    self.assertFalse(accepts([family], changes={selected_job: result}))
        self.assertTrue(accepts(["all"], in_caller=True))
        for family in ("all", "memory", "numeric"):
            for result in ("failure", "cancelled", "skipped", ""):
                with self.subTest(family=family, collision=result):
                    self.assertFalse(accepts([family], changes={"native-cli-collision": result}))
            self.assertFalse(accepts([family], omitted="native-cli-collision"))
        for family in ("all", "language", "native"):
            for result in ("failure", "cancelled", "skipped", ""):
                with self.subTest(family=family, linked=result):
                    self.assertFalse(accepts([family], changes={"canonical-linked-applications": result}))
            self.assertFalse(accepts([family], omitted="canonical-linked-applications"))
        self.assertFalse(accepts(["native"], in_caller=True, changes={"native-plan": "failure"}))
        self.assertFalse(accepts(["cli"], changes={"release-package-dry-run": "success"}))
        self.assertFalse(accepts(["cli"], omitted="native-engine"))
        for invalid in ([], ["misspelled"]):
            self.assertFalse(accepts(invalid))

    def test_manual_measurement_uses_same_selection_and_immutable_products(self):
        impact = job_block(CI, "impact")
        self.assertIn('test "$GITHUB_EVENT_NAME" = pull_request', impact)
        self.assertIn('--base "$PR_BASE" --head "$GITHUB_SHA"', impact)
        self.assertIn('elif test "$GITHUB_EVENT_NAME" = workflow_dispatch', impact)
        self.assertIn('--representative-paths "$REPRESENTATIVE_PATHS"', impact)
        self.assertIn('--expected-revision "$EXPECTED_REVISION"', impact)
        self.assertIn('cat /tmp/ci-impact.json >> "$GITHUB_STEP_SUMMARY"', impact)
        classifier = next(step for step in job_steps(CI, "impact") if "id: classify" in step)
        self.assertIn("shell: bash", classifier)
        self.assertIn("set -euo pipefail", classifier)
        script = textwrap.dedent(classifier.split("        run: |\n", 1)[1])
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        with tempfile.TemporaryDirectory() as directory:
            summary = Path(directory) / "summary"
            script = script.replace("/tmp/ci-impact.json", str(Path(directory) / "impact.json"))
            environment = os.environ | {
                "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_SHA": sha,
                "EXPECTED_REVISION": "b" * 40,
                "REPRESENTATIVE_PATHS": '["src/cli/outcome.rs"]',
                "GITHUB_STEP_SUMMARY": str(summary),
            }
            rejected = subprocess.run(["/bin/bash", "-e", "-c", script], cwd=ROOT,
                                      env=environment, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertFalse(summary.exists())
        self.assertIn('github.event.pull_request.number || github.run_id', CI)
        self.assertIn('group: ci-full-${{ github.run_id }}-', FULL)
        self.assertEqual(FULL.split("\nconcurrency:", 1)[0].count("default: '[\"all\"]'"), 2)

    def test_cancelled_impact_skips_aggregate_gates(self):
        guard = "if: always() && needs.impact.result != 'cancelled'"
        for job in ("browser-compute-canary", "browser-canary", "pr-gate"):
            with self.subTest(job=job):
                self.assertIn(guard, job_block(CI, job))

    def test_architecture_mutations_are_bounded_parallel_shards(self):
        normal = job_block(CI, "static-mutations")
        full = job_block(FULL, "architecture-mutations")
        for block, count, checkout, deadline in (
            (normal, 6, "ref: ${{ github.sha }}", 8),
            (full, 4, FULL_CHECKOUT_REF, 12),
        ):
            with self.subTest(checkout=checkout):
                self.assertIn(f"shard: {list(range(count))}", block)
                self.assertIn(f"timeout-minutes: {deadline}", block)
                self.assertIn(f"--shard-count {count}", block)
                self.assertIn("--shard-index ${{ matrix.shard }}", block)
                self.assertIn("scripts/tests/test_check_memory_runtime.py", block)
                self.assertIn("scripts/tests/test_check_native_host_catalog.py", block)
                self.assertIn("scripts/tests/test_check_production_resident_routing.py", block)
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

    def test_helper_cargo_jobs_inherit_the_installed_toolchain(self):
        for workflow, job in (
            (CI, "static-distribution"),
            (CI, "changed-owner-tests"),
            (FULL, "wasm-bytecode-v1"),
        ):
            with self.subTest(job=job):
                block = job_block(workflow, job)
                # Installing a toolchain does not select it for plain Cargo
                # commands spawned by Python helpers or wasm-pack. A step-local setting
                # also would not survive into the later helper step.
                header = block.split("    steps:\n", 1)[0]
                environment = header.split("    env:\n", 1)[1]
                self.assertEqual(
                    re.findall(r"(?m)^      RUSTUP_TOOLCHAIN: (.+)$", environment),
                    ["nightly-2026-03-03"],
                    "the job must select its toolchain once; duplicate YAML keys invalidate CI",
                )
                self.assertIn(
                    "rustup toolchain install nightly-2026-03-03 --profile minimal",
                    block,
                )

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
        for result in ("BUILD", "SMOKE"):
            self.assertIn(f'test "${result}_RESULT" = success', compute)
        for result in ("CPU", "WGPU", "PARITY"):
            self.assertIn(f'test "${result}_RESULT" = "$expected"', compute)
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

    def test_project_wasm_driver_and_runner_have_separate_finite_budgets(self):
        steps = job_steps(FULL, "project-browser")
        step = next(step for step in steps if "name: Run headless Chrome WASM tests" in step)
        self.assertIn("timeout-minutes: 15", step)
        self.assertIn(
            "WASM_BINDGEN_TEST_WEBDRIVER_JSON: ${{ github.workspace }}/tests/browser/wasm-bindgen-webdriver.json",
            step,
        )
        self.assertIn('WASM_BINDGEN_TEST_TIMEOUT: "180"', step)
        self.assertIn(
            "CHROMEDRIVER_ARGS: --verbose --log-path=${{ runner.temp }}/mech-project-browser-chromedriver.log",
            step,
        )
        configuration = json.loads(
            (ROOT / "tests/browser/wasm-bindgen-webdriver.json").read_text(encoding="utf-8")
        )
        self.assertEqual(configuration, {"timeouts": {"script": 120000}})
        self.assertLess(configuration["timeouts"]["script"], 180 * 1000)
        # Preserve the complete release browser suite and all existing flags.
        command = shlex.split(step.split("        run: |\n", 1)[1].replace("\\\n", " "))
        self.assertEqual(command, [
            "wasm-pack", "test", "--headless", "--chrome", "--release", "src/wasm", "--lib",
            "--no-default-features", "--features", "browser_project,set_union,set_element_of", "--", "--nocapture",
        ])
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("retry", step.split("        run: |\n", 1)[1])
        diagnostic = next(step for step in steps if "name: Retain headless WebDriver failure diagnostics" in step)
        self.assertIn("if: failure()", diagnostic)
        self.assertIn("uses: actions/upload-artifact@v4", diagnostic)
        self.assertIn("path: ${{ runner.temp }}/mech-project-browser-chromedriver.log", diagnostic)

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

    def test_ekf_rejected_replacement_fails_once_without_retrying(self):
        probe = EKF_BROWSER.split(
            'expectedComputeBackend === "wgpu" && applicationQualifiedBeforeReset &&', 1
        )[1].split('const originalSetTimeout', 1)[0]
        self.assertIn('!incompatibleEditAttempted', probe)
        self.assertLess(
            probe.index('incompatibleEditAttempted = true;'),
            probe.index('replaceSource(nextSource)'),
        )
        self.assertIn('if (!incompatibleEditRequested)', probe)
        failure = probe.split('catch (error)', 1)[1]
        self.assertIn('root.dataset.mechPageError = diagnosticText(error)', failure)
        self.assertIn('signalCompletion("ekf-finished")', failure)
        self.assertNotIn('mechDone = "true"', failure)
        self.assertIn('root.dataset.mechObservedIncompatibleEditAttempted', EKF_BROWSER)

    def test_ekf_failure_diagnostics_are_not_numerical_evidence(self):
        self.assertIn('milestones["failureDataset"] = live_dataset', EKF_BROWSER)
        self.assertIn('write_dataset_snapshot(dom_file, live_dataset)', EKF_BROWSER)
        self.assertIn('timeout=5,', EKF_BROWSER)
        self.assertIn('root.dataset.mechObservedContinuitySourceAccepted', EKF_BROWSER)
        self.assertIn('root.dataset.mechObservedReplDiagnostics', EKF_BROWSER)
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

    def test_owner_shards_leave_headroom_for_cold_compilation_and_tests(self):
        block = job_block(CI, "changed-owner-tests")
        settings = block.split("    steps:\n", 1)[0]
        timeout = int(re.search(r"(?m)^    timeout-minutes: (\d+)$", settings).group(1))
        # Runtime already needed 6m11s to build and 5m21s to execute, before
        # checkout/toolchain/dependency setup and post-job cleanup.
        self.assertGreaterEqual(timeout, 25)
        self.assertLessEqual(timeout, 40)
        self.assertIn("fail-fast: false", settings)
        self.assertIn('scripts/run-owner-tests.py --owners "${{ matrix.owners }}"', block)

    def test_contract_commands_select_the_installed_toolchain_for_child_processes(self):
        for workflow, job, command in (
            (CI, "changed-owner-tests", "scripts/run-owner-tests.py"),
            (CI, "static-distribution", "scripts/check-distribution-contracts.py"),
            (FULL, "wasm-bytecode-v1", "wasm-pack test"),
            (FULL, "dynamic-modules", "bash scripts/test-dynamic-modules.sh"),
        ):
            with self.subTest(job=job):
                block = job_block(workflow, job)
                job_settings = block.split("    steps:\n", 1)[0]
                self.assertRegex(
                    job_settings,
                    r"(?m)^      RUSTUP_TOOLCHAIN: nightly-2026-03-03$",
                )
                steps = job_steps(workflow, job)
                install = next(index for index, step in enumerate(steps)
                               if "rustup toolchain install nightly-2026-03-03" in step)
                run = next(index for index, step in enumerate(steps)
                           if command in step)
                self.assertLess(install, run)
                self.assertNotIn("RUSTUP_TOOLCHAIN", steps[run])

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

    def test_pr_full_validation_receives_integration_revision(self):
        block = job_block(CI, "full-validation")
        self.assertIn(
            "validation_ref: ${{ github.sha }}", block
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

    def test_configured_clock_probe_waits_for_a_published_value(self):
        smoke = (ROOT / "scripts/smoke-served-rich-document-browser.sh").read_text()
        body = smoke.split("def assert_console_contract():\n", 1)[1].split("\ndef ", 1)[0]
        function = ast.parse("def assert_console_contract():\n" + body)
        waits = [
            node for node in ast.walk(function)
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
            and node.func.id == "wait_for" and len(node.args) >= 2
            and isinstance(node.args[1], ast.Constant)
            and node.args[1].value == "the configured clock variable after REPL replacement"
        ]
        self.assertEqual(len(waits), 1)
        condition = ast.literal_eval(waits[0].args[0])
        self.assertIn("?.dataset.mechValueAvailable === 'true'", condition)
        self.assertIn("textContent.trim()", condition)

    def test_full_validation_stops_on_cancellation_and_keeps_selected_dependency_gates(self):
        block = job_block(CI, "full-validation")
        condition = re.search(r"(?s)    if: >-\n\s*\$\{\{(.*?)\}\}", block).group(1)
        self.assertIn("!cancelled()", condition)

        def selected(cancelled=False, **overrides):
            values = {
                "needs.impact.outputs.dependent_contracts_required": "true",
                "needs.impact.outputs.docs_only": "false",
                "needs.impact.outputs.windows_canary_required": "true",
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
        self.assertFalse(selected(**{"needs.impact.outputs.dependent_contracts_required": "false"}))
        skipped_optional = {
            "needs.changed-owner-tests.result": "skipped",
            "needs.browser-canary.result": "skipped",
        }
        self.assertTrue(selected(**skipped_optional))
        self.assertFalse(selected(cancelled=True, **skipped_optional))
        skipped_docs = {
            "needs.impact.outputs.docs_only": "true",
            "needs.impact.outputs.windows_canary_required": "false",
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

    def test_all_validation_checkouts_record_the_selected_revision(self):
        for workflow, requested, kind in (
            (CI, "${{ github.sha }}", "${{ github.event_name == 'pull_request' && 'integration-merge' || 'exact-ref' }}"),
            (FULL, "${{ inputs.validation_ref || github.sha }}", "exact-ref"),
        ):
            jobs = checkout_jobs(workflow)
            self.assertTrue(jobs)
            for job in jobs:
                with self.subTest(job=job, kind=kind):
                    steps = job_steps(workflow, job)
                    self.assertEqual([i for i, step in enumerate(steps)
                                      if "uses: actions/checkout@" in step], [0])
                    self.assertIn(f"ref: {requested}", steps[0])
                    self.assertIn(f"VALIDATION_KIND: {kind}", steps[1])
                    self.assertIn(f"REQUESTED_REF: {requested}", steps[1])
                    self.assertIn("CHECKOUT_SHA: ${{ steps.checkout.outputs.commit }}", steps[1])
                    self.assertIn('$RECORD_CHECKOUT_IDENTITY', steps[1])
                    self.assertIn("shell: bash", steps[1])

    def test_checkout_identity_recorder_is_fail_closed_and_auditable(self):
        self.assertEqual(CHECKOUT_IDENTITY, workflow_checkout_identity(FULL))
        for validation_kind in ("integration-merge", "exact-ref"):
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

    def test_type_memory_boundary_is_unwaived(self):
        type_memory = "python3 scripts/check-type-memory-boundary.py"
        unit = "scripts/tests/test_check_type_memory_boundary.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(type_memory, block)
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

    def test_type_system_is_unwaived_and_full_conformance_is_owned(self):
        type_memory = "python3 scripts/check-type-memory-boundary.py"
        type_system = "python3 scripts/check-type-system.py"
        unit = "scripts/tests/test_check_type_system.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(type_memory, block)
            self.assertIn(type_system, block)
            self.assertLess(block.index(type_memory), block.index(type_system))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)
        full = job_block(FULL, "architecture-contracts")
        for target in (
            "type_system_builtin",
            "type_system_solver",
            "type_system_conversion",
            "matrix_set_conversion",
            "type_system_catalog",
            "type_system_source",
        ):
            self.assertIn(target, full)

    def test_semantic_type_authority_is_unwaived_and_full_conformance_is_owned(self):
        type_system = "python3 scripts/check-type-system.py"
        semantic_authority = "python3 scripts/check-semantic-type-authority.py"
        unit = "scripts/tests/test_check_semantic_type_authority.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(type_system, block)
            self.assertIn(semantic_authority, block)
            self.assertLess(block.index(type_system), block.index(semantic_authority))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)
        full = job_block(FULL, "architecture-contracts")
        self.assertIn("Execute the complete semantic type authority contract", full)
        self.assertGreaterEqual(full.count("--test canonical_type_contract"), 3)

    def test_managed_memory_and_miri_are_required_gates(self):
        memory_planner = "python3 scripts/check-memory-planner.py"
        memory_runtime = "python3 scripts/check-memory-runtime.py"
        unit = "scripts/tests/test_check_memory_runtime.py"
        for block in (
            normal_static_contracts(),
            full_architecture_contracts(),
        ):
            self.assertIn(memory_planner, block)
            self.assertIn(memory_runtime, block)
            self.assertLess(block.index(memory_planner), block.index(memory_runtime))
            self.assertIn(unit, block)
            self.assertNotIn("continue-on-error", block)

        runtime = job_block(FULL, "managed-memory-runtime")
        fixed = job_block(FULL, "managed-memory-fixed-profiles")
        miri = job_block(FULL, "managed-memory-miri")
        cargo = job_block(FULL, "cargo")
        self.assertIn(FULL_CHECKOUT_REF, runtime)
        self.assertIn(FULL_CHECKOUT_REF, fixed)
        self.assertIn(FULL_CHECKOUT_REF, miri)
        self.assertIn("--test memory_runtime", runtime)
        self.assertIn("--test memory_safety", runtime)
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
        self.assertIn("--test managed_functions", fixed)
        self.assertIn("miri test --locked", miri)
        self.assertIn(safety_features, miri)
        self.assertIn("- managed-memory-runtime", cargo)
        self.assertIn("- managed-memory-fixed-profiles", cargo)
        self.assertIn("- managed-memory-miri", cargo)

    def test_managed_memory_retains_resident_live_and_execution_unit_tests(self):
        runtime = job_block(FULL, "managed-memory-runtime")
        commands = [
            " ".join(line.split())
            for line in runtime.replace("\\\n", " ").splitlines()
        ]
        for filter in (
            "resident::general::live::",
            "resident::general::execution::tests::",
            "resident::native_semantics::",
        ):
            self.assertIn(
                "cargo +nightly-2026-03-03 test --locked -p mech-engine "
                "--no-default-features --features full_compiler,resident-artifact "
                f"--lib {filter}",
                commands,
            )
        self.assertNotIn("continue-on-error", runtime)

    def test_artifact_completeness_prefetches_before_offline_native_build(self):
        block = job_block(FULL, "artifact-closure")
        fetch = "cargo fetch --locked"
        closure = "python3 scripts/check-artifact-completeness.py ${{ matrix.representative }}"
        self.assertIn(fetch, block)
        self.assertLess(block.index(fetch), block.index(closure))

    def test_dynamic_modules_prefetches_before_offline_native_build(self):
        block = job_block(FULL, "dynamic-modules")
        fetch = "cargo fetch --locked"
        smoke = "bash scripts/test-dynamic-modules.sh"
        self.assertIn(fetch, block)
        self.assertLess(block.index(fetch), block.index(smoke))

    def test_syntax_profiles_prefetch_and_run_exhaustive_streaming_once(self):
        block = job_block(FULL, "cargo-language")
        fetch = "cargo fetch --locked"
        self.assertIn(fetch, block)
        full = "cargo +nightly-2026-03-03 test --locked -p mech-syntax --lib --tests --no-default-features --features full"
        base = "cargo +nightly-2026-03-03 test --locked -p mech-syntax --no-default-features --features base"
        for command in (full, base):
            self.assertIn(command, block)
            self.assertLess(block.index(fetch), block.index(command))
        self.assertEqual(block.count("-p mech-syntax --lib --tests"), 1)
        self.assertIn("--test canonical_document_feature_gates", block)
        self.assertIn("for suite in canonical_document_root canonical_base_contracts", block)
        self.assertNotIn("--test document_streaming_complexity", block)
        self.assertNotIn("--test document_streaming_equivalence", block)
        self.assertNotIn("continue-on-error", block)

    def test_syntax_grammar_step_uses_current_targets_and_features(self):
        block = job_block(FULL, "cargo-language")
        grammar = next(step for step in job_steps(FULL, "cargo-language")
                       if "Run reduced syntax feature profiles" in step)
        self.assertNotRegex(grammar, r"--test\s+grammar_conformance\b")
        for target in (
            "canonical_rule_registry",
            "canonical_recursive_core_inventory", "canonical_recursive_core_schema",
            "canonical_grammar",
        ):
            self.assertIn(f"--test {target}", grammar)
            self.assertTrue((ROOT / f"src/syntax/tests/{target}.rs").is_file())
        for target in ("canonical_mechdown_closed_rules", "canonical_executable_conformance", "canonical_executable_recovery"):
            self.assertIn(target, grammar)
            self.assertTrue((ROOT / f"src/syntax/tests/canonical_rules/{target}.rs").is_file())
        self.assertIn('--lib "document::parser::canonical::tests::$suite"', grammar)
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
