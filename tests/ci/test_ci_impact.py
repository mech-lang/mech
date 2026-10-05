#!/usr/bin/env python3

import importlib.util
import os
import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location("ci_impact", SCRIPTS / "ci-impact.py")
assert SPEC and SPEC.loader
CI_IMPACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CI_IMPACT)
OWNERS = CI_IMPACT.load_owners()


class ImpactClassifierTests(unittest.TestCase):
    def classify(self, paths, labels=()):
        return CI_IMPACT.classify(paths, labels, OWNERS)

    def test_documentation_only_changes_compile_nothing(self):
        result = self.classify(["docs/distributions.md", "LICENSE"])
        self.assertTrue(result["docs_only"])
        self.assertFalse(result["standard_canaries_required"])
        self.assertFalse(result["windows_canary_required"])
        self.assertEqual(result["changed_owners"], [])

    def test_documentation_inside_owned_code_trees_compile_nothing(self):
        for path in ("src/core/README.md", "hosts/browser/README.md"):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertTrue(result["docs_only"])
                self.assertEqual(result["changed_owners"], [])
                self.assertEqual(result["owner_shards"], [])
                self.assertFalse(result["browser_canary_required"])

    def test_machine_change_runs_managed_function_behavior(self):
        result = self.classify(["machines/math/src/add.rs"])
        self.assertEqual(result["changed_owners"], ["machine-functions"])
        command = OWNERS["machine-functions"]["command"]
        self.assertIn("mech-stdlib", command)
        self.assertNotIn("mech-math", command)
        self.assertTrue(result["standard_canaries_required"])
        self.assertTrue(result["browser_canary_required"])
        self.assertFalse(result["cross_cutting_standard_suite_required"])

    def test_cross_cutting_change_selects_all_runnable_standard_owners(self):
        result = self.classify(["src/core/src/value.rs"])
        expected = sorted(
            name
            for name, owner in OWNERS.items()
            if owner["standard"] and owner["command"]
        )
        self.assertEqual(result["changed_owners"], expected)
        self.assertTrue(result["cross_cutting_standard_suite_required"])
        self.assertTrue(result["browser_canary_required"])
        self.assertEqual(len(result["owner_shards"]), len(expected))

    def test_cross_cutting_keeps_directly_changed_nonstandard_owners(self):
        result = self.classify(["src/core/src/value.rs", "hosts/robot-arm/src/lib.rs"])
        self.assertIn("mech-robot-arm", result["changed_owners"])
        self.assertIn("mech-core", result["changed_owners"])
        result = self.classify(["src/core/src/value.rs", "machines/math/src/ops/add.rs"])
        self.assertIn("mech-stdlib", result["changed_owners"])
        self.assertNotIn("machine-functions", result["changed_owners"])

    def test_browser_related_change_requests_browser_canary(self):
        result = self.classify(["hosts/scene/src/lib.rs"])
        self.assertTrue(result["browser_canary_required"])

    def test_browser_capable_shared_hosts_request_browser_canary(self):
        for path in (
            "hosts/console/src/lib.rs",
            "hosts/time/src/lib.rs",
            "hosts/timer/src/lib.rs",
        ):
            with self.subTest(path=path):
                self.assertTrue(self.classify([path])["browser_canary_required"])

    def test_nbody_validation_surface_requests_browser_canary(self):
        for path in (
            "examples/n-body/n-body.mec",
            "examples/n-body/mech.mcfg",
            "scripts/check-nbody-physics-reference.py",
            "scripts/smoke-served-resident-nbody-browser.sh",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertIn("nbody-browser-canary", result["matched_owners"])
                self.assertTrue(result["browser_canary_required"])

    def test_ekf_validation_surface_requests_browser_canary(self):
        for path in (
            "examples/ekf/localization.mec",
            "examples/ekf/mech.mcfg",
            "scripts/check-ekf-browser-results.py",
            "scripts/smoke-served-resident-ekf-browser.sh",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertIn("ekf-browser-canary", result["matched_owners"])
                self.assertTrue(result["browser_canary_required"])

    def test_leading_dot_owner_paths_are_not_treated_as_unknown(self):
        for path in (".github/workflows/ci.yml", "./.github/workflows/ci.yml"):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertEqual(result["matched_owners"], ["architecture-contracts", "ci-tools"])
                self.assertEqual(result["unmatched_paths"], [])
                self.assertTrue(result["changed_owners"])
                self.assertTrue(result["cross_cutting_standard_suite_required"])

    def test_full_label_requests_full_validation_for_ordinary_changes(self):
        ordinary = self.classify(["machines/math/src/add.rs"])
        requested = self.classify(["machines/math/src/add.rs"], ["ci:full"])
        self.assertFalse(ordinary["full_validation_required"])
        self.assertTrue(requested["full_validation_required"])
        requested = self.classify(["hosts/terminal/src/lib.rs"], ["ci:full"])
        self.assertTrue(requested["browser_canary_required"])
        self.assertTrue(requested["browser_applications_required"])
        self.assertEqual(set(requested["contract_families"]), set(CI_IMPACT.CONTRACT_FAMILIES))

    def test_machine_changes_share_one_behavior_owner(self):
        paths = [f"machines/{package}/src/lib.rs" for package in (
            "math", "compare", "logic", "range", "matrix", "set", "string",
            "stats", "combinatorics",
        )]
        for changed in ([path] for path in paths):
            self.assertEqual(self.classify(changed)["changed_owners"], ["machine-functions"])
        combined = self.classify(paths)
        self.assertEqual(combined["changed_owners"], ["machine-functions"])
        self.assertEqual(len(combined["owner_shards"]), 1)
        command = OWNERS["machine-functions"]["command"]
        self.assertIn("managed_functions", command)
        self.assertNotIn("--exact", command)
        self.assertNotIn("machine-functions", self.classify(["src/core/src/lib.rs"])["changed_owners"])

    def test_architecture_contract_changes_require_full_validation(self):
        for path in (
            ".github/workflows/ci-full.yml",
            ".github/workflows/ci-native-plan.yml",
            "scripts/check-operation-contract.py",
            "scripts/check-type-memory-boundary.py",
            "scripts/tests/test_check_type_memory_boundary.py",
            "scripts/check-type-system.py",
            "scripts/tests/test_check_type_system.py",
            "scripts/check-semantic-type-authority.py",
            "scripts/tests/test_check_semantic_type_authority.py",
            "tests/architecture/program-artifact/v1.json",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertIn("architecture-contracts", result["matched_owners"])
                self.assertTrue(result["full_validation_required"])
                self.assertTrue(result["cross_cutting_standard_suite_required"])
                self.assertTrue(result["browser_canary_required"])
        for path in (
            "README.md",
            "docs/design/type-memory-boundary.md",
            "docs/design/type-system-v1.md",
            "docs/design/v0.4-endgame.md",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertTrue(result["docs_only"])
                self.assertFalse(result["full_validation_required"])

    def test_docs_only_change_can_still_request_full_validation(self):
        result = self.classify(["docs/distributions.md"], ["ci:full"])
        self.assertTrue(result["docs_only"])
        self.assertTrue(result["full_validation_required"])
        self.assertFalse(result["static_contracts_required"])
        self.assertFalse(result["standard_canaries_required"])
        self.assertEqual(result["owner_shards"], [])

    def test_unknown_paths_are_handled_conservatively(self):
        result = self.classify(["new-top-level-area/file.rs"])
        self.assertTrue(result["cross_cutting_standard_suite_required"])
        self.assertEqual(result["unmatched_paths"], ["new-top-level-area/file.rs"])

    def test_every_owner_gets_an_independent_runner(self):
        names = [f"owner-{index:02}" for index in range(31)]
        shards = CI_IMPACT.make_shards(names)
        self.assertEqual(len(shards), len(names))
        flattened = sorted(
            owner
            for shard in shards
            for owner in shard["owners"].split(",")
        )
        self.assertEqual(flattened, names)

    def test_safety_and_abi_owners_require_complete_contracts(self):
        for path in (
            "src/core/src/types.rs", "src/engine/src/source_semantics/frontend.rs",
            "src/runtime/src/program.rs", "src/bytecode/src/lib.rs",
            "src/build/src/lib.rs", "Cargo.toml", "Cargo.lock",
            "src/syntax/src/lib.rs", "src/compute/src/lib.rs",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertTrue(result["dependent_contracts_required"])
                self.assertTrue(result["contract_families"])
        for path in ("src/core/src/types.rs", "src/bytecode/src/lib.rs", "Cargo.toml", "Cargo.lock", "unknown/product.rs"):
            self.assertTrue(self.classify([path])["full_validation_required"])

    def test_narrow_changes_do_not_run_native_or_windows_qualification(self):
        for path in ("hosts/terminal/src/lib.rs",):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertFalse(result["full_validation_required"])
                self.assertFalse(result["windows_canary_required"])
                self.assertTrue(result["standard_canaries_required"])

    def test_large_browser_application_cases_follow_affected_owners(self):
        for path in ("examples/ekf/localization.mec", "hosts/gpu/src/lib.rs", "src/engine/src/lib.rs"):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertTrue(result["browser_applications_required"])
                self.assertTrue(result["browser_canary_required"])
        self.assertTrue(self.classify(["machines/math/src/ops/add.rs"])["browser_applications_required"])
        self.assertFalse(self.classify(["hosts/time/src/lib.rs"])["browser_applications_required"])
        self.assertFalse(self.classify(["docs/distributions.md"])["browser_applications_required"])

    def test_syntax_owner_keeps_compact_growth_and_stream_contracts(self):
        command = OWNERS["mech-syntax"]["command"]
        for target in ("document_streaming_complexity", "document_streaming_boundaries",
                       "document_streaming_lifecycle", "document_streaming_resources",
                       "document_streaming_storage"):
            self.assertIn(target, command)
        self.assertIn("--lib", command)
        self.assertEqual(command[-3:], ["--", "--skip", "large_"])

    def test_authoritative_syntax_inputs_are_never_prose_only(self):
        for path in (
            "docs/design/specification.mec",
            "docs/design/grammar-audit/ports.tsv",
            "docs/design/grammar-audit/canonical-dependencies.tsv",
            "docs/design/grammar-audit/recursive-core.tsv",
            "docs/design/grammar-audit/recursive-core-certification.tsv",
            "docs/design/grammar-audit/canonical-syntax-schema.json",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertIn("mech-syntax", result["matched_owners"])
                self.assertFalse(result["docs_only"])
                self.assertTrue(result["static_contracts_required"])
                self.assertIn("language", result["contract_families"])
                self.assertNotIn("distribution", result["contract_families"])
                self.assertIn("mech-syntax", result["changed_owners"])
        self.assertTrue(self.classify(["docs/design/grammar-audit/new-input.data"])["full_validation_required"])
        self.assertTrue(self.classify(["docs/design/grammar-audit/README.md"])["docs_only"])

    def test_documentation_owner_cannot_mask_an_explicit_product_owner(self):
        owners = {
            "docs": {"name": "docs", "paths": ["docs/**"], "command": [],
                     "standard": False, "cross_cutting": False, "docs": True},
            "spec": {"name": "spec", "paths": ["docs/schema.md"], "command": ["check"],
                     "standard": True, "cross_cutting": True, "full": True},
        }
        result = CI_IMPACT.classify(["docs/schema.md"], [], owners)
        self.assertFalse(result["docs_only"])
        self.assertTrue(result["full_validation_required"])
        self.assertEqual(result["changed_owners"], ["spec"])

    def test_numeric_implementation_keeps_complete_oracles_without_release_qualification(self):
        for path in ("machines/math/src/ops/add.rs", "machines/matrix/src/ops/transpose.rs"):
            result = self.classify([path])
            self.assertEqual(result["contract_families"], ["artifact", "linkage", "numeric"])
            self.assertTrue(result["browser_applications_required"])
            self.assertFalse(result["native_validation_required"])
            self.assertFalse(result["full_validation_required"])
        for path in ("machines/math/src/catalog.rs", "machines/math/build.rs"):
            self.assertTrue(self.classify([path])["native_validation_required"])
        self.assertTrue(self.classify(["machines/math/Cargo.toml"])["full_validation_required"])
        self.assertTrue(self.classify(["machines/math/README.md"])["docs_only"])

    def test_cli_presentation_keeps_command_contracts_without_native_generation(self):
        result = self.classify(["src/cli/commands/repl/presentation.rs"])
        self.assertEqual(result["contract_families"], ["cli"])
        self.assertFalse(result["native_validation_required"])
        self.assertFalse(result["browser_canary_required"])
        self.assertFalse(result["full_validation_required"])
        self.assertEqual(result["changed_owners"], [])

    def test_cli_product_paths_select_the_affected_products(self):
        for path in ("src/serve.rs", "src/cli/commands/format/mod.rs", "src/cli/bundle_web.rs"):
            result = self.classify([path])
            self.assertEqual(result["contract_families"], ["browser", "cli"])
            self.assertTrue(result["browser_canary_required"])
            self.assertFalse(result["native_validation_required"])
        result = self.classify(["src/cli/commands/build.rs"])
        self.assertTrue(result["native_validation_required"])
        self.assertIn("linkage", result["contract_families"])
        self.assertFalse(result["full_validation_required"])

    def test_cli_authority_keeps_artifact_memory_runtime_and_native_contracts(self):
        for path in ("src/cli/canonical_source.rs", "src/cli/host_grants.rs",
                     "src/cli/commands/repl/session.rs", "src/cli/commands/run.rs"):
            result = self.classify([path])
            self.assertTrue({"artifact", "memory", "runtime", "linkage", "native", "browser"}
                            <= set(result["contract_families"]))
            self.assertTrue(result["browser_applications_required"])
            self.assertNotIn("distribution", result["contract_families"])

    def test_root_product_and_native_fixture_changes_run_their_contracts(self):
        for path in ("tests/mech_build.rs", "tests/fixtures/native-linkage/src/main.rs",
                     "tests/fixtures/native-live-host/Cargo.toml"):
            self.assertTrue(self.classify([path])["native_validation_required"])
        self.assertIn("browser", self.classify(["tests/mech_serve.rs"])["contract_families"])

    def test_runtime_acceptance_tests_do_not_expand_to_every_owner(self):
        result = self.classify(["src/runtime/tests/source_acceptance_cases/numeric.rs"])
        self.assertEqual(result["changed_owners"], [])
        self.assertEqual(result["contract_families"], ["runtime"])
        self.assertFalse(result["cross_cutting_standard_suite_required"])
        self.assertFalse(result["full_validation_required"])

    def test_runtime_test_files_delegate_to_complete_runtime_family(self):
        self.assertEqual(OWNERS["runtime-tests"]["command"], [])
        for path in ("src/runtime/tests/canonical_document_outputs.rs",
                     "src/runtime/tests/canonical_mixed_resource_planning.rs",
                     "src/runtime/tests/recording_bench_facade.rs"):
            result = self.classify([path])
            self.assertEqual(result["changed_owners"], [])
            self.assertEqual(result["contract_families"], ["runtime"])
        result = self.classify(["src/runtime/tests/resident_external_contract.rs"])
        self.assertIn("architecture", result["contract_families"])

    def test_dependent_owners_do_not_promote_every_change_to_full(self):
        result = self.classify(["src/syntax/src/document/parser/canonical/mod.rs"])
        self.assertIn("mech-core", result["changed_owners"])
        self.assertFalse(result["full_validation_required"])
        self.assertEqual(result["contract_families"], ["artifact", "browser", "language"])
        self.assertTrue(result["browser_applications_required"])
        combined = self.classify(["src/cli/commands/repl/presentation.rs", "machines/math/src/ops/add.rs"])
        self.assertEqual(combined["contract_families"], ["artifact", "cli", "linkage", "numeric"])

    def test_ordinary_mec_documentation_is_not_an_unknown_product(self):
        for path in ("docs/reference/matrix.mec", "docs/getting-started/repl.mec", "docs/mechdown/table.mec"):
            result = self.classify([path])
            self.assertTrue(result["docs_only"])
            self.assertEqual(result["owner_shards"], [])
            self.assertFalse(result["dependent_contracts_required"])
        self.assertFalse(self.classify(["docs/design/specification.mec"])["docs_only"])
        self.assertFalse(self.classify(["docs/design/grammar-audit/README.mec"])["docs_only"])
        self.assertFalse(self.classify(["tests/fixtures/format_static/main.mec"])["docs_only"])

    def test_manual_measurement_rejects_invalid_identity_and_paths(self):
        sha = "a" * 40
        self.assertEqual(CI_IMPACT.representative_paths('["src/cli/outcome.rs"]', sha, sha), ["src/cli/outcome.rs"])
        for ref in ("main", "", "b" * 40):
            with self.assertRaises(ValueError):
                CI_IMPACT.representative_paths('["src/cli/outcome.rs"]', ref, sha)
        for encoded in ('[]', '{}', '[1]', '["/tmp/file"]', '["../source.rs"]', '["src//file"]', '["src/./file"]', 'null'):
            with self.subTest(encoded=encoded), self.assertRaises(ValueError):
                CI_IMPACT.representative_paths(encoded, sha, sha)
        result = subprocess.run(
            [sys.executable, str(SCRIPTS / "ci-impact.py"), "--representative-paths",
             '["src/cli/outcome.rs"]', "--expected-revision", sha],
            env=os.environ | {"GITHUB_EVENT_NAME": "pull_request"}, capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"only in an explicit manual measurement", result.stderr)


if __name__ == "__main__":
    unittest.main()
