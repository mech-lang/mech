#!/usr/bin/env python3

import importlib.util
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

    def test_machine_change_runs_r6_catalog_integration_not_machine_private_tests(self):
        result = self.classify(["machines/math/src/add.rs"])
        self.assertEqual(result["changed_owners"], ["mech-math"])
        command = OWNERS["mech-math"]["command"]
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
                self.assertEqual(result["matched_owners"], ["ci-tools"])
                self.assertEqual(result["unmatched_paths"], [])
                self.assertEqual(result["changed_owners"], [])
                self.assertFalse(result["cross_cutting_standard_suite_required"])

    def test_full_label_requests_full_validation_for_ordinary_changes(self):
        ordinary = self.classify(["machines/math/src/add.rs"])
        requested = self.classify(["machines/math/src/add.rs"], ["ci:full"])
        self.assertFalse(ordinary["full_validation_required"])
        self.assertTrue(requested["full_validation_required"])

    def test_machine_changes_select_the_r6_catalog_owner_suite(self):
        for owner in (
            "mech-math",
            "mech-compare",
            "mech-logic",
            "mech-range",
            "mech-matrix",
            "mech-set",
            "mech-string",
            "mech-stats",
            "mech-combinatorics",
        ):
            package = owner.removeprefix("mech-")
            with self.subTest(owner=owner):
                result = self.classify([f"machines/{package}/src/lib.rs"])
                self.assertEqual(result["changed_owners"], [owner])
                command = OWNERS[owner]["command"]
                self.assertIn("r6_managed_functions", command)
                self.assertIn(
                    "catalog_inventory_is_classified_into_r6_implementation_families",
                    command,
                )

    def test_architecture_contract_changes_require_full_validation(self):
        for path in (
            ".github/workflows/ci-full.yml",
            ".github/workflows/ci-native-plan.yml",
            "scripts/check-operation-contract.py",
            "scripts/check-r2-type-memory-boundary.py",
            "scripts/tests/test_check_r2_type_memory_boundary.py",
            "scripts/check-r3-type-system.py",
            "scripts/tests/test_check_r3_type_system.py",
            "scripts/check-r4-type-cutover.py",
            "scripts/tests/test_check_r4_type_cutover.py",
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
            "docs/design/ROADMAP.mec",
            "docs/design/v0.4-endgame.md",
        ):
            with self.subTest(path=path):
                result = self.classify([path])
                self.assertIn("architecture-contracts", result["matched_owners"])
                self.assertTrue(result["full_validation_required"])

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


class RegisteredReviewTests(unittest.TestCase):
    def test_registered_slice_runs_static_checks_and_its_focused_regressions(self):
        result = CI_IMPACT.classify(["src/engine/src/source_semantics/frontend.rs"], ["ci:full"], OWNERS, "review")
        self.assertEqual(result["matched_owners"], ["mech-engine"])
        self.assertEqual(result["changed_owners"], ["s8-review-regressions"])
        self.assertTrue(result["static_contracts_required"])
        self.assertTrue(result["review_only"])
        for field in ["standard_canaries_required", "windows_canary_required", "browser_canary_required", "cross_cutting_standard_suite_required", "full_validation_required"]:
            self.assertFalse(result[field], field)

    def test_review_ci_change_does_not_fan_out_to_all_product_owners(self):
        result = CI_IMPACT.classify([".github/workflows/ci-full.yml"], [], OWNERS, "review")
        self.assertEqual(result["changed_owners"], ["s8-review-regressions"])
        self.assertFalse(result["full_validation_required"])
        self.assertTrue(result["static_contracts_required"])

    def test_unknown_review_paths_fail_closed_to_normal_qualification(self):
        result = CI_IMPACT.classify(["unowned-area/file.rs"], ["ci:full"], OWNERS, "review")
        self.assertFalse(result["review_only"])
        self.assertTrue(result["standard_canaries_required"])
        self.assertTrue(result["full_validation_required"])

    def test_landing_always_retains_full_product_qualification(self):
        for paths in [["src/runtime/src/input.rs"], ["docs/distributions.md"], []]:
            result = CI_IMPACT.classify(paths, [], OWNERS, "landing")
            self.assertTrue(result["landing_candidate"])
            self.assertFalse(result["review_only"])
            self.assertFalse(result["docs_only"])
            for field in ["standard_canaries_required", "windows_canary_required", "browser_canary_required", "full_validation_required"]:
                self.assertTrue(result[field], field)
            self.assertIn("mech-engine", result["changed_owners"])
            self.assertNotIn("s8-review-regressions", result["changed_owners"])

    def test_role_requires_exact_registered_number_branch_and_repository(self):
        from ci_review_slices import review_role
        self.assertEqual(review_role(844, "codex/syntax-s8r04-dynamic-binding", "mech-lang/mech", base="codex/syntax-s8r03-bound-schemas")[0], "review")
        self.assertEqual(review_role(830, "codex/syntax-s8c-cutover", "mech-lang/mech")[0], "landing")
        for number, branch, repo in [
            (999, "codex/syntax-s8r04-dynamic-binding", "mech-lang/mech"),
            (844, "codex/syntax-s8r04-copy", "mech-lang/mech"),
            (844, "codex/syntax-s8r04-dynamic-binding", "someone/mech"),
            (830, "codex/syntax-s8r04-dynamic-binding", "mech-lang/mech"),
        ]:
            self.assertEqual(review_role(number, branch, repo)[0], "ordinary")

    def test_registry_has_one_landing_and_no_empty_review_checks(self):
        import json
        from ci_review_slices import REGISTRY, review_role
        registry = json.loads(REGISTRY.read_text())
        self.assertNotIn(str(registry["landing"]["number"]), registry["slices"])
        for number, entry in registry["slices"].items():
            role, selected = review_role(number, entry["branch"], registry["repository"], base=entry["base"])
            self.assertEqual(role, "review")
            for command in selected["checks"]:
                self.assertEqual(command[0], "cargo")
                self.assertIn(command[2], ["test", "check"])
                self.assertIn("--locked", command)
        registry["slices"]["844"]["checks"] = []
        with self.assertRaises(ValueError):
            review_role(844, "codex/syntax-s8r04-dynamic-binding", registry["repository"], registry, base=registry["slices"]["844"]["base"])

    def test_interactive_registration_is_reachable_from_numeric_pr_event(self):
        from ci_review_slices import review_role
        role, entry = review_role(837, "codex/syntax-s8e7-interactive", "mech-lang/mech", base="codex/syntax-s8e6-graph-planning")
        self.assertEqual(role, "review")
        self.assertTrue(any("interactive::tests::" in command for command in entry["checks"]))

    def test_registry_rejects_nonnumeric_keys_and_preserves_minimal_config_check(self):
        import json
        from ci_review_slices import REGISTRY, review_role
        registry = json.loads(REGISTRY.read_text())
        checks = registry["slices"]["831"]["checks"]
        self.assertTrue(any(command[command.index("--features") + 1] == "source"
                            and "config_profile" in command for command in checks))
        registry["slices"]["None"] = registry["slices"].pop("837")
        with self.assertRaises(ValueError):
            review_role(837, "codex/syntax-s8e7-interactive", "mech-lang/mech", registry)

    def test_review_retargeting_cannot_keep_reduced_validation(self):
        import json
        from ci_review_slices import REGISTRY, review_role
        registry = json.loads(REGISTRY.read_text())
        for number, entry in registry["slices"].items():
            self.assertEqual(review_role(number, entry["branch"], registry["repository"], base=entry["base"])[0], "review")
            for target in ("integration/v0.4", "main", "unexpected/parent", ""):
                with self.subTest(number=number, target=target):
                    role, checks = review_role(number, entry["branch"], registry["repository"], base=target)
                    self.assertEqual(role, "ordinary")
                    self.assertIsNone(checks)
                    result = CI_IMPACT.classify(["src/core/src/value.rs"], [], OWNERS, role)
                    self.assertFalse(result["review_only"])
                    self.assertTrue(result["standard_canaries_required"])
                    self.assertTrue(result["browser_canary_required"])
                    self.assertNotIn("s8-review-regressions", result["changed_owners"])
                    self.assertTrue(CI_IMPACT.classify(["src/core/src/value.rs"], ["ci:full"], OWNERS, role)["full_validation_required"])
        for target in ("integration/v0.4", registry["slices"]["844"]["branch"], "unexpected/parent"):
            role, _ = review_role(830, registry["landing"]["branch"], registry["repository"], base=target)
            self.assertEqual(role, "landing")
            self.assertTrue(CI_IMPACT.classify([], [], OWNERS, role)["full_validation_required"])

    def test_event_passes_base_to_both_role_consumers(self):
        import json
        import os
        from unittest.mock import patch
        from ci_review_slices import REGISTRY, environment_role
        env = {"PR_NUMBER": "844", "PR_HEAD_REF": "codex/syntax-s8r04-dynamic-binding", "PR_HEAD_REPOSITORY": "mech-lang/mech", "PR_BASE_REF": "integration/v0.4", "PR_BASE_SHA": "a" * 40}
        registry = json.loads(REGISTRY.read_text())
        with patch("ci_review_slices.trusted_registry_from_revision", return_value=registry):
            with patch.dict(os.environ, env, clear=True):
                self.assertEqual(environment_role()[0], "ordinary")
                os.environ["PR_BASE_REF"] = "codex/syntax-s8r03-bound-schemas"
                self.assertEqual(environment_role()[0], "review")
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(workflow.count("PR_BASE_REF: ${{ github.event.pull_request.base.ref }}"), 2)
        self.assertEqual(workflow.count("PR_BASE_SHA: ${{ github.event.pull_request.base.sha }}"), 2)

    def test_review_registry_is_loaded_from_the_trusted_base_revision(self):
        import json
        from types import SimpleNamespace
        from unittest.mock import Mock
        from ci_review_slices import REGISTRY, trusted_registry_from_revision
        registry = json.loads(REGISTRY.read_text())
        run = Mock(return_value=SimpleNamespace(stdout=json.dumps(registry)))
        revision = "b" * 40

        self.assertEqual(trusted_registry_from_revision(revision, run), registry)
        run.assert_called_once_with(
            ["git", "show", f"{revision}:.github/ci/s8-review-slices.json"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIsNone(trusted_registry_from_revision("HEAD", run))
        self.assertEqual(run.call_count, 1)

    def test_review_gate_requires_focused_success_and_does_not_claim_full_success(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        review_gate = workflow.split('if test "$REVIEW_ONLY" = true')[1].split('elif test "$DOCS_ONLY"')[0]
        self.assertIn('test "$OWNER_RESULT" = success', review_gate)
        self.assertIn('test "$LINUX_RESULT" = skipped', review_gate)
        self.assertIn('test "$FULL_RESULT" = skipped', workflow)
        self.assertIn("test ! -e src/syntax/src/parser.rs", workflow)
        self.assertIn("tests/fixtures/full-source-runtime/Cargo.toml", workflow)


if __name__ == "__main__":
    unittest.main()
