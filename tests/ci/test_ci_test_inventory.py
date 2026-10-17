"""Dynamically reject unowned canonical integration targets and weak profiles."""

import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from ci_test_inventory import (
    cargo_test_command, cfg_predicate, crate_cfgs, feature_closure,
    integration_targets, inventory_gaps, target_covered, workflow_commands,
)


class IntegrationInventoryTests(unittest.TestCase):
    def test_every_canonical_target_has_a_ci_execution_profile(self):
        self.assertEqual(inventory_gaps(), [])

    def command(self, tail):
        return cargo_test_command(["cargo", "+nightly-2026-03-03", "test", "-p", "sample", *tail])

    def test_compilation_and_listing_do_not_supply_execution_credit(self):
        for arguments in (["--test", "fresh", "--no-run"], ["--test", "fresh", "--", "--list"]):
            self.assertIsNone(self.command(arguments))
        self.assertIsNone(cargo_test_command(["cargo", "check", "-p", "sample"]))

    def test_folded_and_literal_commands_are_extracted_without_comment_decoys(self):
        source = '''steps:
  - name: cargo test -p counterfeit --test decoy
    env:
      DECOY: cargo test -p counterfeit --test decoy
    run: >-
      cargo +nightly-2026-03-03 test -p sample
      --no-default-features --features full --test folded
  - run: |
      # cargo test -p counterfeit --test decoy
      echo "cargo test -p counterfeit --test decoy"
      cargo test -p sample \\
        --test literal
'''
        commands = list(workflow_commands(source))
        self.assertEqual([command["targets"] for command in commands], [["folded"], ["literal"]])

    def test_feature_closure_does_not_union_independent_ci_commands(self):
        manifest = {"features": {"default": ["minimal"], "full": ["source"], "source": [], "native": []}}
        self.assertEqual(feature_closure(manifest, self.command(["--features", "full", "--no-default-features"])), {"full", "source"})
        self.assertIn("minimal", feature_closure(manifest, self.command([])))
        self.assertIn("native", feature_closure(manifest, self.command(["--all-features"])))

    def test_only_the_maintained_stage_wrapper_can_supply_indirect_execution(self):
        source = '''steps:
  - run: |
      python3 scripts/run-native-plan-stage.py --stage fixture --timeout-secs 20 -- cargo test -p sample --test actual
      python3 print_decoy.py -- cargo test -p sample --test decoy
'''
        self.assertEqual([command["targets"] for command in workflow_commands(source)], [["actual"]])

    def test_disabled_or_allowed_failure_steps_and_jobs_cannot_own_targets(self):
        for clause in ("if: false", "if: ${{ false }}", "continue-on-error: true"):
            for metadata in (f"    {clause}\n", ""):
                step_clause = "" if metadata else f"        {clause}\n"
                source = f"jobs:\n  sample:\n{metadata}    steps:\n      - name: disabled\n{step_clause}        run: cargo test -p sample --test absent\n"
                with self.subTest(clause=clause, job=bool(metadata)):
                    self.assertEqual(list(workflow_commands(source)), [])

    def test_recorded_executables_require_real_execution_of_the_same_receipt(self):
        compile_step = '''steps:
  - run: |
      python3 scripts/run-native-plan-stage.py --stage compile --record-test-executable fresh --test-executable-file exact-receipt -- cargo test -p sample --features full --test fresh --no-run --message-format=json-render-diagnostics
'''
        self.assertEqual(list(workflow_commands(compile_step)), [])
        for receipt, harness, count in (("different-receipt", "--test-threads=1", 0), ("exact-receipt", "--list", 0), ("exact-receipt", "--test-threads=1", 1)):
            source = compile_step + f"  - run: python3 scripts/run-native-plan-stage.py --stage execute --test-executable-file {receipt} -- {harness}\n"
            self.assertEqual(len(list(workflow_commands(source))), count)

    def test_boolean_cfg_features_and_unknown_platform_predicates(self):
        self.assertTrue(cfg_predicate('all(feature = "full", not(feature = "minimal"))', {"full"}))
        self.assertFalse(cfg_predicate('all(feature = "full", feature = "native")', {"full"}))
        self.assertTrue(cfg_predicate('any(feature = "full", feature = "native")', {"full"}))
        self.assertIsNone(cfg_predicate('all(feature = "full", target_os = "macos")', {"full"}))

    def test_cfg_comments_and_source_literal_decoys_are_ignored(self):
        source = '// #![cfg(feature = "decoy")]\n#![cfg(feature = "full")]\nconst TEXT: &str = r##"#![cfg(feature = "literal-decoy")]"##;'
        self.assertEqual(list(crate_cfgs(source)), ['feature = "full"'])

    def test_new_targets_require_their_own_selection_and_one_adequate_profile(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "tests").mkdir()
            path = directory / "tests/fresh.rs"
            path.write_text('#![cfg(all(feature = "source", feature = "native"))]\n#[test]\nfn executes() {}\n')
            manifest = {"features": {"source": [], "native": []}}
            target = integration_targets(directory, manifest)["fresh"]
            source = self.command(["--test", "fresh", "--features", "source"])
            native = self.command(["--test", "fresh", "--features", "native"])
            adequate = self.command(["--test", "fresh", "--features", "source,native"])
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [source, native]))
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [adequate]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--lib", "--all-features"])]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "other", "--all-features"])]))
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--tests", "--all-features"])]))
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--all-features"])]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--all-features", "unrelated::unit::module"])]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--all-features", "--", "--skip", "executes"])]))
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--all-features", "--", "--exact", "executes"])]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--bin=fixture", "--all-features"])]))

    def test_explicit_manifest_targets_retain_required_features(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            path = directory / "custom.rs"
            path.write_text("#[test]\nfn custom() {}\n")
            manifest = {"package": {"autotests": False}, "features": {"native": []}, "test": [{"name": "named", "path": "custom.rs", "required-features": ["native"]}]}
            targets = integration_targets(directory, manifest)
            self.assertEqual(set(targets), {"named"})
            self.assertFalse(target_covered("sample", "named", targets["named"], manifest, [self.command(["--test", "named"])]))
            self.assertTrue(target_covered("sample", "named", targets["named"], manifest, [self.command(["--test", "named", "--features=native"])]))


if __name__ == "__main__":
    unittest.main()
