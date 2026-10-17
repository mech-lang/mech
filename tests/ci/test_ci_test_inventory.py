"""Dynamically reject unowned canonical integration targets and weak profiles."""

import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from ci_test_inventory import (
    cargo_test_command, cfg_predicate, crate_cfgs, feature_closure,
    integration_targets, inventory_gaps, invoked_generator_commands,
    registered_cases, target_covered, workflow_commands, workflow_truth,
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
        for clause in (
            "if: false", "if: ${{ false }}", "if: ${{ 0 }}", "if: ${{ -0 }}",
            "if: ${{ 0.0 }}", "if: ${{ 0e3 }}", "if: ${{ 0x0 }}",
            "if: ${{ null }}", "if: ${{ '' }}", 'if: "${{ 0 }}"',
            "if: ${{ !true }}", "if: ${{ false || 0 }}",
            "if: ${{ github.event_name && false }}",
            "if: ${{ 1 == 2 }}", "if: ${{ 1 > 2 }}", "if: ${{ null != 0 }}",
            "if: ${{ 'zero' > 0 }}", "if: ${{ 'a' != 'A' }}",
            "if: ${{ false && success() }}", "if: ${{ false && contains(github.ref, 'x') }}",
            "if: ${{ contains('needs.decoy', 'absent') }}",
            "if: ${{ fromJSON('false') }}",
            "continue-on-error: true", "continue-on-error: ${{ true }}",
            "continue-on-error: ${{ '#' }} # a real YAML comment",
            "continue-on-error: ${{ matrix.optional }}",
        ):
            for metadata in (f"    {clause}\n", ""):
                step_clause = "" if metadata else f"        {clause}\n"
                source = f"jobs:\n  sample:\n{metadata}    steps:\n      - name: disabled\n{step_clause}        run: cargo test -p sample --test absent\n"
                with self.subTest(clause=clause, job=bool(metadata)):
                    self.assertEqual(list(workflow_commands(source)), [])

    def test_workflow_truthiness_preserves_strings_and_dynamic_conditions(self):
        for value in ("true", "${{ -2 }}", "${{ 0x1 }}", "${{ 'false' }}", "${{ '0' }}", "${{ ' ' }}", "${{ !false }}", "${{ true || github.event_name }}", "${{ 2 >= 1 }}", "${{ 'a' == 'A' }}", "${{ 0 == false }}", "${{ always() }}"):
            self.assertIs(workflow_truth(value), True, value)
        for value in ("${{ github.event_name }}", "${{ success() }}", "${{ matrix.optional }}", "${{ (github.event_name || 'fallback') == 'fallback' }}"):
            self.assertIsNone(workflow_truth(value), value)
        source = "jobs:\n  sample:\n    if: ${{ 'false' }}\n    steps:\n      - run: cargo test -p sample --test actual\n"
        self.assertEqual([command["targets"] for command in workflow_commands(source)], [["actual"]])
        for source in (
            "jobs:\n  sample:\n    if: >-\n      ${{ 0 }}\n    steps:\n      - run: cargo test -p sample --test absent\n",
            "steps:\n  - if: ${{ 0 }}\n    run: cargo test -p sample --test absent\n",
            "steps:\n  - name: disabled\n    if: |\n      ${{ false ||\n          0 }}\n    run: cargo test -p sample --test absent\n",
        ):
            self.assertEqual(list(workflow_commands(source)), [], source)
        source = "steps:\n  - run: |\n      if: false\n      cargo test -p sample --test actual\n"
        self.assertEqual([command["targets"] for command in workflow_commands(source)], [["actual"]], "run content is not step metadata")

    def test_generator_commands_require_a_reachable_entry_point_call(self):
        literal = repr(["cargo", "test", "-p", "sample", "--test", "fresh"])
        runner = f"run_streamed({literal})"
        sources = (
            (f"def unused():\n    {runner}\ndef main():\n    return 0\n", False),
            (f"def main():\n    if False:\n        {runner}\n", False),
            (f"def helper():\n    {runner}\ndef main():\n    if False:\n        helper()\n", False),
            (f"def main():\n    return 0\n    {runner}\n", False),
            (f"def main():\n    if True:\n        return 0\n    {runner}\n", False),
            (f"def main():\n    False and {runner}\n", False),
            (f"def main():\n    True or {runner}\n", False),
            (f"def main():\n    deferred = lambda: {runner}\n", False),
            (f"def main():\n    for value in []:\n        {runner}\n", False),
            (f"def main():\n    while True:\n        pass\n    {runner}\n", False),
            (f"def main():\n    assert False\n    {runner}\n", False),
            (f"def main():\n    match False:\n        case True:\n            {runner}\n", False),
            (f"def main():\n    try:\n        pass\n    except ValueError:\n        {runner}\n", False),
            (f"def main():\n    command = {literal}\n    command.append('--no-run')\n    run_streamed(command)\n", False),
            (f"def main():\n    command = {literal}\n    if dynamic():\n        command = ['echo', 'decoy']\n    run_streamed(command)\n", False),
            (f"def helper():\n    {runner}\ndef main():\n    helper()\n", True),
            (f"def helper():\n    command = {literal}\n    return run_streamed(command)\ndef main():\n    with context():\n        helper()\n", True),
            (f"def main():\n    if True:\n        {runner}\n", True),
            (f"def main():\n    if False:\n        return 0\n    else:\n        {runner}\n", True),
        )
        for relative in ("scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                path = root / relative
                path.parent.mkdir()
                workflow = f"steps:\n  - run: python3 {relative}\n"
                for source, expected in sources:
                    path.write_text(source + "\nif __name__ == '__main__':\n    raise SystemExit(main())\n")
                    with self.subTest(script=relative, source=source):
                        commands = list(invoked_generator_commands(root, [workflow]))
                        self.assertEqual([command["targets"] for command in commands], [["fresh"]] if expected else [])
                        self.assertEqual(list(invoked_generator_commands(root, [])), [])
                path.write_text(f"def main():\n    {runner}\n")
                self.assertEqual(list(invoked_generator_commands(root, [workflow])), [], "an uncalled main is not an entry point")

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

    def test_raw_string_cfg_values_use_rust_delimiters(self):
        for literal in ('r"full"', 'r#"full"#', 'r###"full"###'):
            for predicate in (f'feature = {literal}', f'all(test, feature = {literal})'):
                self.assertTrue(cfg_predicate(predicate, {"full"}))
                self.assertFalse(cfg_predicate(predicate, set()))
        self.assertTrue(cfg_predicate('feature = r##"quoted"name"##', {'quoted"name'}))
        source = '#![cfg_attr(feature = r#"full"#, cfg(feature = r##"native"##))]\n'
        predicates = list(crate_cfgs(source))
        self.assertTrue(any(cfg_predicate(predicate, {"full"}) is False for predicate in predicates))
        self.assertTrue(all(cfg_predicate(predicate, {"full", "native"}) is True for predicate in predicates))

    def test_conditional_crate_attributes_cannot_disable_the_selected_profile(self):
        for attribute in (
            '#![cfg_attr(feature = "full", cfg(any()))]',
            '#![r#cfg_attr(r#all(feature = "full"), r#cfg(r#any()))]',
            '#![cfg_attr(feature = "full", cfg_attr(feature = "source", cfg(any())))]',
            '#![cfg_attr(feature = "full", allow(dead_code), cfg(any()))]',
        ):
            predicates = list(crate_cfgs(attribute + "\n#[test]\nfn actual() {}\n"))
            self.assertTrue(predicates)
            self.assertTrue(all(cfg_predicate(predicate, set()) is not False for predicate in predicates))
            self.assertTrue(any(cfg_predicate(predicate, {"full", "source"}) is False for predicate in predicates))

    def test_helper_module_cfg_does_not_disable_an_unconditional_crate(self):
        source = '#![allow(dead_code)]\nmod helper {\n#![cfg(feature = "internal")]\n}\n#[test]\nfn actual() {}\n'
        self.assertEqual(list(crate_cfgs(source)), [])
        self.assertEqual(list(crate_cfgs('#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]\n#[test]\nfn actual() {}\n')), [])

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

    def test_skip_filters_accept_both_harness_spellings_and_full_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fresh.rs"
            path.write_text("mod cases { #[test] fn only_case() {} }\n")
            target = {"path": path, "required-features": []}
            for skips in (["--skip", "only_case"], ["--skip=only_case"], ["--skip=cases::"], ["--skip="]):
                with self.subTest(skips=skips):
                    self.assertFalse(target_covered("sample", "fresh", target, {}, [self.command(["--test", "fresh", "--", *skips])]))
            self.assertTrue(target_covered("sample", "fresh", target, {}, [self.command(["--test", "fresh", "--", "--exact", "cases::only_case"])]))
            self.assertFalse(target_covered("sample", "fresh", target, {}, [self.command(["--test", "fresh", "--", "--exact", "only_case"])]))

    def test_enclosing_module_gates_follow_each_case(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fresh.rs"
            target = {"path": path, "required-features": []}
            manifest = {"features": {"native": [], "full": []}}
            modules = (
                '#[cfg(feature = "native")] mod cases { #[test] fn actual() {} }',
                'pub(crate) mod cases { #![cfg(feature = "native")] #[test] fn actual() {} }',
                '#[cfg_attr(not(feature = "native"), cfg(any()))] mod cases { #[test] fn actual() {} }',
                '#[cfg(feature = "native")] mod parent { mod child { #[test] fn actual() {} } }',
                'mod parent { #![cfg(feature = "native")] mod child { #[test] fn actual() {} } }',
                '#[cfg(feature = r#"native"#)] mod r#cases { #[r#test] fn r#actual() {} }',
            )
            for source in modules:
                path.write_text(source)
                with self.subTest(source=source):
                    self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh"])]))
                    self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh", "--features=native"])]))
            path.write_text('#[cfg(feature = "native")] mod gated { #[test] fn actual() {} }\nmod ordinary { #[test] fn actual() {} }\n')
            self.assertEqual([case["name"] for case in registered_cases(path.read_text())], ["gated::actual", "ordinary::actual"])
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh"])]))
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh", "gated::"])]))
            path.write_text('#[cfg(feature = "full")] mod parent { #[cfg(feature = "native")] mod child { #[test] fn actual() {} } }')
            self.assertFalse(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh", "--features=full"]), self.command(["--test", "fresh", "--features=native"])]))
            self.assertTrue(target_covered("sample", "fresh", target, manifest, [self.command(["--test", "fresh", "--features=full,native"])]))

    def test_ignored_only_selection_cannot_own_an_ordinary_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "ordinary.rs"
            path.write_text("#[test]\nfn actual() {}\n")
            target = {"path": path, "required-features": []}
            self.assertFalse(target_covered("sample", "ordinary", target, {}, [self.command(["--test", "ordinary", "--", "--ignored"])]))
            self.assertTrue(target_covered("sample", "ordinary", target, {}, [self.command(["--test", "ordinary"])]))

    def test_ignored_registration_order_and_conditional_ignores_match_the_harness(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "qualification.rs"
            target = {"path": path, "required-features": []}
            manifest = {"features": {"full": []}}
            for attributes in (
                '#[ignore = "linked qualification"]\n#[test]',
                '#[test]\n#[ignore = "linked qualification"]',
                '#[r#ignore = "linked qualification"]\n#[r#test]',
                '#[r#test]\n#[r#ignore = "linked qualification"]',
                '#[ignore = "linked qualification"]\n#[doc(alias = ["nested", "metadata"])]\n#[test]',
                '#[test]\n#[doc(alias = ["nested", "metadata"])]\n#[ignore = "linked qualification"]',
                '#[cfg_attr(feature = "full", ignore)]\n#[test]',
                '#[test]\n#[cfg_attr(feature = "full", ignore)]',
            ):
                path.write_text(f"{attributes}\nfn actual() {{}}\n")
                for harness, expected in (([], False), (["--ignored"], True), (["--include-ignored"], True)):
                    command = self.command(["--test", "qualification", "--features", "full", "--", *harness])
                    self.assertEqual(target_covered("sample", "qualification", target, manifest, [command]), expected, attributes)


if __name__ == "__main__":
    unittest.main()
