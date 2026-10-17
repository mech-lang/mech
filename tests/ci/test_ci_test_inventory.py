"""Dynamically reject unowned canonical integration targets and weak profiles."""

import ast
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
    workflow_sources, reachable_generator_commands,
)


class IntegrationInventoryTests(unittest.TestCase):
    def test_every_canonical_target_has_a_ci_execution_profile(self):
        self.assertEqual(inventory_gaps(), [])

    def command(self, tail):
        return cargo_test_command(["cargo", "+nightly-2026-03-03", "test", "-p", "sample", *tail])

    def test_compilation_and_listing_do_not_supply_execution_credit(self):
        for arguments in (
            ["--test", "fresh", "--no-run"], ["--test", "fresh", "--", "--list"],
            ["--test", "fresh", "--", "--bench"],
            ["--test", "fresh", "--", "--bench", "--test"],
            ["--test", "fresh", "--", "--help"],
            ["--test", "fresh", "--help"],
            ["--test", "fresh", "--", "--exclude-should-panic"],
        ):
            self.assertIsNone(self.command(arguments))
        self.assertIsNotNone(self.command(["--test", "fresh", "--", "--test", "--test-threads=1", "--color", "always"]))
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

    def test_cargo_output_only_and_unmodeled_options_cannot_own_tests(self):
        for options in (
            ["--unit-graph"], ["-Z", "unstable-options", "--unit-graph"],
            ["--unit-graph=true"], ["--config", "target.host.runner='/bin/true'"],
            ["--target", "host", "--config=target.host.runner='/bin/true'"],
            ["--unknown-mode"], ["--profile"], ["--target="], ["--exclude"],
        ):
            with self.subTest(options=options):
                self.assertIsNone(self.command(["--test", "fresh", *options]))
        self.assertIsNotNone(self.command(["--test=fresh", "--locked", "--offline", "-j1",
                                           "--target", "host", "--profile=test", "--message-format=json"]))

    def test_manifest_paths_must_select_the_reviewed_root_workspace(self):
        for spelling in ("--manifest-path", "--manifest-path="):
            for path, accepted in (("Cargo.toml", True), ("./Cargo.toml", True),
                                   ("nested/Cargo.toml", False), ("../Cargo.toml", False),
                                   ("/tmp/other/Cargo.toml", False), ("", False)):
                options = [spelling, path] if spelling == "--manifest-path" else [spelling + path]
                with self.subTest(options=options):
                    self.assertEqual(self.command(["--test", "fresh", *options]) is not None, accepted)
        source = "steps:\n  - run: python3 scripts/run-native-plan-stage.py --stage test -- cargo test -p sample --manifest-path nested/Cargo.toml --test fresh\n"
        self.assertEqual(list(workflow_commands(source)), [])

    def test_workspace_exclusions_override_explicit_package_selections(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "tests").mkdir()
            (directory / "tests/fresh.rs").write_text("#[test]\nfn actual() {}\n")
            manifest = {}
            target = integration_targets(directory, manifest)["fresh"]
            for spelling in ("--exclude", "--exclude="):
                for pattern, accepted in (("sample", False), ("sam*", False),
                                           ("s?mple", False), ("other", True)):
                    options = [spelling, pattern] if spelling == "--exclude" else [spelling + pattern]
                    command = self.command(["--workspace", "--test", "fresh", *options])
                    self.assertEqual(target_covered("sample", "fresh", target, manifest, [command]), accepted)
            self.assertIsNone(self.command(["--workspace", "--exclude=sample@1.0"]))

    def test_only_the_maintained_stage_wrapper_can_supply_indirect_execution(self):
        source = '''steps:
  - run: |
      python3 scripts/run-native-plan-stage.py --stage fixture --timeout-secs 20 -- cargo test -p sample --test actual
  - run: python3 print_decoy.py -- cargo test -p sample --test decoy
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
        source = "steps:\n  - run: |\n      echo 'if: false'\n      cargo test -p sample --test actual\n"
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

    def test_generator_list_mutations_and_aliases_invalidate_execution_evidence(self):
        literal = repr(["cargo", "test", "-p", "sample", "--test", "fresh"])
        for mutation in (
            "command[:] = ['true']", "command[0] = 'true'", "del command[1:]",
            "command[0:1] += ['true']", "command += ['--no-run']",
            "alias = command\nalias[:] = ['true']", "alias = command\ndel alias[1:]",
            "alias = command\nalias[0] = 'true'", "alias = command\nalias.append('--no-run')",
            "alias = command\nalias += ['--no-run']", "alias: list = command\nalias[0] = 'true'",
            "aliases = [command]\naliases[0][:] = ['true']",
            "environment = {}\nenvironment['command'] = command\nenvironment['command'][:] = ['true']",
            "environment = {}\nenvironment['command']: list = command\nenvironment['command'][:] = ['true']",
            "aliases = []\naliases += [command]\naliases[0][:] = ['true']",
            "(alias := command)\nalias[0] = 'true'",
            "alias, = [command]\nalias[:] = ['true']",
            "def mutate():\n    command[:] = ['true']\nmutate()",
            f"def mutate():\n    command[:] = ['true']\nmutate()\ncommand = {literal}\nmutate()",
            "if dynamic():\n    command[:] = ['true']",
        ):
            source = f"command = {literal}\n{mutation}\nrun_streamed(command)\n"
            with self.subTest(mutation=mutation):
                self.assertEqual(list(reachable_generator_commands(ast.parse(source))), [])
        for mutation in (
            "environment = {}\nenvironment['MODE'] = 'checked'\ndel environment['MODE']",
            "alias = command\nalias = ['true']\nalias[:] = ['false']",
            "alias = command\ndel alias",
        ):
            source = f"command = {literal}\n{mutation}\nrun_streamed(command)\n"
            self.assertEqual([command["targets"] for command in reachable_generator_commands(ast.parse(source))], [["fresh"]])

    def test_generator_exit_calls_terminate_the_entry_point_including_helpers(self):
        runner = "run_streamed(['cargo', 'test', '-p', 'sample', '--test', 'fresh'])"
        for exit_call in ("sys.exit(0)", "exit(0)", "quit(0)", "os._exit(0)",
                          "system.exit(0)", "stop(0)"):
            imports = "import sys as system\nfrom sys import exit as stop\n"
            for body in (f"{exit_call}\n{runner}\n",
                         f"def die():\n    {exit_call}\ndie()\n{runner}\n"):
                self.assertEqual(list(reachable_generator_commands(ast.parse(imports + body))), [], body)
        for source in (f"{runner}\nsys.exit(0)\n", f"sys.exit({runner})\n",
                       f"if False:\n    sys.exit(0)\n{runner}\n",
                       f"def helper():\n    return 0\nhelper()\n{runner}\n"):
            self.assertEqual([command["targets"] for command in reachable_generator_commands(ast.parse(source))], [["fresh"]])

    def test_real_maintained_generators_keep_their_literal_test_commands(self):
        for relative in ("scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"):
            module = ast.parse((ROOT / relative).read_text())
            commands = list(reachable_generator_commands(module))
            self.assertTrue(commands, relative)
            self.assertTrue(any("mech-build" in command["packages"]
                                and "generate_native_projects" in command["targets"] for command in commands), relative)

    def test_recorded_executables_require_real_execution_of_the_same_receipt(self):
        compile_step = '''steps:
  - run: |
      python3 scripts/run-native-plan-stage.py --stage compile --record-test-executable fresh --test-executable-file exact-receipt -- cargo test -p sample --features full --test fresh --no-run --message-format=json-render-diagnostics
'''
        self.assertEqual(list(workflow_commands(compile_step)), [])
        for receipt, harness, count in (("different-receipt", "--test-threads=1", 0), ("exact-receipt", "--list", 0), ("exact-receipt", "--test-threads=1", 1)):
            source = compile_step + f"  - run: python3 scripts/run-native-plan-stage.py --stage execute --test-executable-file {receipt} -- {harness}\n"
            self.assertEqual(len(list(workflow_commands(source))), count)

    def test_shell_control_and_termination_cannot_supply_direct_or_recorded_credit(self):
        command = "cargo test -p sample --test absent"
        scripts = (
            f"if false; then\n  {command}\nfi",
            f"if false\nthen\n  {command}\nfi",
            f"if true; then\n  true\nelse\n  {command}\nfi",
            f"for item in; do\n  {command}\ndone",
            f"while false; do\n  {command}\ndone",
            f"unused() {{\n  {command}\n}}",
            f": <<'DECOY'\n{command}\nDECOY",
            f"text='\n{command}\n'",
            f"false &&\n  {command}",
            f"true ||\n  {command}",
            f"exit 0\n{command}",
            f"MODE=unused exit 0\n{command}",
            f"exec true\n{command}",
            f"return 0\n{command}",
            f"! exit 0\n{command}",
            f"time exit 0\n{command}",
            f"set -n\n{command}",
            f"set -o noexec\n{command}",
            f"shopt -s expand_aliases\nalias cargo=true\n{command}",
            f"hash -p /bin/true cargo\n{command}",
            f"export PATH=/tmp/decoy\n{command}",
            f"PATH=/tmp/decoy\n{command}",
            f"PATH=/tmp/decoy {command}",
            f"cd /tmp/decoy\n{command}",
            f"unset PATH\n{command}",
            f"eval 'exit 0'\n{command}",
            f"trap 'exit 0' DEBUG\n{command}",
            f"`echo exit` 0\n{command}",
            f"$COMMAND\n{command}",
            f"{command}\nif false; then\n  true\nfi",
            f"{command} $EXTRA_FLAGS",
            f"{command} ${{{{ inputs.flags }}}}",
        )
        for script in scripts:
            source = "steps:\n  - run: |\n" + "\n".join("      " + line for line in script.splitlines())
            with self.subTest(script=script):
                self.assertEqual(list(workflow_commands(source)), [])
        source = "steps:\n  - run: |\n      set -euo pipefail\n      MODE=checked cargo test -p sample --test actual\n      exit 0\n      cargo test -p sample --test absent\n  - run: cargo test -p sample --test next_step\n"
        self.assertEqual([command["targets"] for command in workflow_commands(source)], [["actual"], ["next_step"]])
        source = "steps:\n  - run: |\n      python3 scripts/run-native-plan-stage.py --stage compile --record-test-executable fresh --test-executable-file receipt -- cargo test -p sample --test fresh --no-run\n  - run: |\n      exit 0\n      python3 scripts/run-native-plan-stage.py --stage execute --test-executable-file receipt -- --test-threads=1\n"
        self.assertEqual(list(workflow_commands(source)), [])

    def test_generator_invocation_must_be_on_an_admitted_shell_path(self):
        literal = repr(["cargo", "test", "-p", "sample", "--test", "fresh"])
        script = f"def main():\n    run_streamed({literal})\nif __name__ == '__main__':\n    main()\n"
        for relative in ("scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                path = root / relative
                path.parent.mkdir()
                path.write_text(script)
                for invocation in (f"python3 {relative}", f"python3 scripts/run-native-plan-stage.py --stage generator --timeout-secs 20 -- python3 {relative}"):
                    for body, expected in (
                        (invocation, True),
                        (f"set -euo pipefail\n{invocation}", True),
                        (f"if false; then\n  {invocation}\nfi", False),
                        (f"exit 0\n{invocation}", False),
                        (f"exec true\n{invocation}", False),
                        (f"hash -p /bin/true python3\n{invocation}", False),
                        (f"rm -f /home/runner/.cargo/bin/cargo\nln -s /bin/true /home/runner/.cargo/bin/cargo\n{invocation}", False),
                        (f"unused() {{\n  {invocation}\n}}", False),
                        (f": <<'DECOY'\n{invocation}\nDECOY", False),
                    ):
                        source = "steps:\n  - run: |\n" + "\n".join("      " + line for line in body.splitlines())
                        with self.subTest(relative=relative, body=body):
                            commands = list(invoked_generator_commands(root, [source]))
                            self.assertEqual([command["targets"] for command in commands], [["fresh"]] if expected else [])

    def test_only_supported_commands_can_precede_cargo_or_recorded_execution(self):
        command = "cargo test -p sample --test absent"
        for prefix in (
            "rm -f /home/runner/.cargo/bin/cargo\nln -s /bin/true /home/runner/.cargo/bin/cargo",
            "cp /bin/true /home/runner/.cargo/bin/cargo",
            "mv /tmp/fake /home/runner/.cargo/bin/cargo",
            "install /bin/true /home/runner/.cargo/bin/cargo",
            "python3 arbitrary_mutator.py",
            "cargo install fake-cargo --force",
            "python3 scripts/run-native-plan-stage.py --stage replace --timeout-secs 20 -- rm -f /home/runner/.cargo/bin/cargo",
            "printf -v PATH /tmp/decoy",
            "MODE=${BASH_CMDS[cargo]:=/bin/true}",
        ):
            body = prefix + "\n" + command
            source = "steps:\n  - run: |\n" + "\n".join("      " + line for line in body.splitlines())
            with self.subTest(prefix=prefix):
                self.assertEqual(list(workflow_commands(source)), [])
        compile_step = "steps:\n  - run: python3 scripts/run-native-plan-stage.py --stage compile --record-test-executable fresh --test-executable-file receipt -- cargo test -p sample --test fresh --no-run\n"
        for body in (
            "rm -f /home/runner/.cargo/bin/cargo\npython3 scripts/run-native-plan-stage.py --stage execute --test-executable-file receipt -- --test-threads=1",
            "python3 scripts/run-native-plan-stage.py --stage execute --test-executable-file receipt -- --bench",
        ):
            source = compile_step + "  - run: |\n" + "\n".join("      " + line for line in body.splitlines())
            self.assertEqual(list(workflow_commands(source)), [])

    def test_step_and_inherited_working_directory_and_shell_must_be_admitted(self):
        command = "cargo test -p sample --test actual"
        for directory in ("nested", "../other", "/tmp/other", "${{ matrix.checkout }}"):
            for source in (
                f"steps:\n  - run: {command}\n    working-directory: {directory}\n",
                f"defaults:\n  run:\n    working-directory: {directory}\nsteps:\n  - run: {command}\n",
                f"jobs:\n  sample:\n    steps:\n      - run: {command}\n    defaults:\n      run:\n        working-directory: {directory}\n",
            ):
                with self.subTest(source=source):
                    self.assertEqual(list(workflow_commands(source)), [])
        for directory in (".", "./", "./.", "${{ github.workspace }}", "${{github.workspace}}/."):
            source = f"jobs:\n  sample:\n    defaults:\n      run:\n        working-directory: nested\n    steps:\n      - run: {command}\n        working-directory: {directory}\n"
            self.assertEqual([entry["targets"] for entry in workflow_commands(source)], [["actual"]])
        for setting in ("shell: bash -n {0}", "working-directory: nested"):
            source = f"steps:\n  - uses: ./.github/actions/absent\n    {setting}\n"
            from ci_test_inventory import local_action_invocations
            self.assertEqual(list(local_action_invocations(source)), [])
        source = f"defaults:\n  run:\n    shell: bash -n {{0}}\nsteps:\n  - run: {command}\n"
        self.assertEqual(list(workflow_commands(source)), [])

    def test_job_gates_cover_the_complete_mapping_in_any_key_order(self):
        for clause in ("if: false", "if: ${{ 0 }}", "continue-on-error: true", "continue-on-error: ${{ matrix.optional }}"):
            source = f"jobs:\n  disabled:\n    steps:\n      - run: cargo test -p sample --test absent\n    {clause}\n  enabled:\n    steps:\n      - run: cargo test -p sample --test actual\n    if: true\n"
            with self.subTest(clause=clause):
                self.assertEqual([entry["targets"] for entry in workflow_commands(source)], [["actual"]])

    def test_run_properties_inside_metadata_or_scalar_text_never_execute(self):
        decoy = "run: cargo test -p sample --test absent"
        for metadata in (
            "env:\n  DECOY: |\n    " + decoy + "\n",
            "name: |\n  " + decoy + "\n",
            "env:\n  DECOY:\n    " + decoy + "\n",
            "env:\n  DECOY: >-\n    steps:\n      - " + decoy + "\n",
        ):
            source = metadata + "steps:\n  - run: cargo test -p sample --test actual\n"
            with self.subTest(metadata=metadata):
                self.assertEqual([entry["targets"] for entry in workflow_commands(source)], [["actual"]])
        source = "steps:\n  - name: |\n      " + decoy + "\n    env:\n      MESSAGE: |\n        " + decoy + "\n    run: cargo test -p sample --test actual\n"
        self.assertEqual([entry["targets"] for entry in workflow_commands(source)], [["actual"]])

    def test_ambiguous_or_recursive_yaml_cannot_supply_ownership(self):
        for source in (
            "steps:\n  - run: cargo test -p sample --test absent\n    run: true\n",
            "steps: &recursive\n  - env:\n      LOOP: *recursive\n    run: cargo test -p sample --test absent\n",
            "steps:\n  - !!python/object:decoy\n    run: cargo test -p sample --test absent\n",
            "steps: [this is not a step mapping]\n",
        ):
            self.assertEqual(list(workflow_commands(source)), [])

    def test_composite_action_credit_requires_invocation_and_effective_inputs(self):
        action = '''inputs:
  suite:
    default: engine
runs:
  using: composite
  steps:
    - if: inputs.suite == 'engine'
      shell: bash
      run: cargo test -p sample --test engine
    - if: ${{ inputs.suite == 'hosted' }}
      shell: bash
      run: cargo test -p sample --test hosted
'''
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / ".github/actions/select/action.yaml"
            path.parent.mkdir(parents=True)
            path.write_text(action)
            workflow = root / ".github/workflows/test.yml"
            workflow.parent.mkdir()
            for steps, expected in (
                ("  - run: true", []),
                ("  - uses: ./.github/actions/select", ["engine"]),
                ("  - uses: ./.github/actions/select\n    with:\n      suite: hosted", ["hosted"]),
                ("  - uses: ./.github/actions/select\n    with:\n      suite: ''", []),
                ("  - uses: ./.github/actions/select\n    with:\n      suite: ${{ matrix.suite }}", []),
                ("  - uses: ./.github/actions/select\n    if: false", []),
                ("  - uses: ./.github/actions/select\n    continue-on-error: true", []),
                ("  - uses: ./.github/actions/select\n    with:\n      suite: engine\n      suite: hosted", []),
                ("  - run: |\n      uses: ./.github/actions/select", []),
                ("  - run: |\n      steps:\n        - uses: ./.github/actions/select", []),
            ):
                workflow.write_text("steps:\n" + steps + "\n")
                with self.subTest(steps=steps):
                    commands = [command for source in workflow_sources(root) for command in workflow_commands(source)]
                    self.assertEqual([command["targets"] for command in commands], [[name] for name in expected])
            workflow.write_text("jobs:\n  disabled:\n    if: ${{ 0 }}\n    steps:\n      - uses: ./.github/actions/select\n")
            self.assertEqual(len(workflow_sources(root)), 1)
            workflow.write_text("steps:\n  - uses: ./.github/actions/select\n")
            path.write_text(action.replace("using: composite", "using: node24"))
            self.assertEqual(len(workflow_sources(root)), 1)
            path.write_text("inputs:\n  missing:\n    required: true\nruns:\n  using: composite\n  steps:\n    - run: cargo test -p sample --test absent\n      shell: bash\n")
            self.assertEqual(len(workflow_sources(root)), 1, "a missing required input cannot own an unconditional target")

    def test_composite_run_steps_require_explicit_supported_shells(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow = root / ".github/workflows/test.yml"
            workflow.parent.mkdir(parents=True)
            workflow.write_text("steps:\n  - uses: ./.github/actions/local\n")
            action = root / ".github/actions/local/action.yml"
            action.parent.mkdir(parents=True)
            for shell, accepted in (("", False), ("      shell: bash\n", True),
                                    ("      shell: sh\n", True), ("      shell: bash --noprofile -n {0}\n", False)):
                action.write_text("runs:\n  using: composite\n  steps:\n    - run: cargo test -p sample --test fresh\n" + shell)
                commands = [command for source in workflow_sources(root) for command in workflow_commands(source)]
                self.assertEqual([command["targets"] for command in commands], [["fresh"]] if accepted else [])
            action.write_text("runs:\n  using: composite\n  steps:\n    - run: cargo test -p sample --test fresh\n      shell: bash\n    - run: true\n")
            self.assertEqual([command for source in workflow_sources(root) for command in workflow_commands(source)], [],
                             "one missing shell invalidates the entire action")
        self.assertEqual(len(list(workflow_commands("steps:\n  - run: cargo test -p sample --test fresh\n"))), 1)

    def test_nested_composite_actions_propagate_inputs_without_recursion_credit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflow = root / ".github/workflows/test.yml"
            workflow.parent.mkdir(parents=True)
            workflow.write_text("steps:\n  - uses: ./.github/actions/outer\n    with:\n      suite: hosted\n")
            outer = root / ".github/actions/outer/action.yml"
            outer.parent.mkdir(parents=True)
            outer.write_text('''inputs:
  suite:
    default: engine
runs:
  using: composite
  steps:
    - uses: ./.github/actions/inner
      with:
        suite: ${{ inputs.suite }}
''')
            inner = root / ".github/actions/inner/action.yml"
            inner.parent.mkdir()
            inner.write_text('''runs:
  using: composite
  steps:
    - if: inputs.suite == 'hosted'
      shell: bash
      run: cargo test -p sample --test actual
    - if: inputs.suite == 'engine'
      shell: bash
      run: cargo test -p sample --test absent
    - uses: ./.github/actions/outer
      with:
        suite: engine
''')
            sources = workflow_sources(root)
            self.assertEqual(len(sources), 3)
            self.assertEqual([command["targets"] for source in sources for command in workflow_commands(source)], [["actual"]])

    def test_inventory_does_not_accept_uninvoked_action_or_wrong_input_profile(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for ordinal, relative in enumerate(("src/core", "src/syntax", "src/engine", "src/stdlib", "src/runtime", "src/build", "hosts/gpu")):
                directory = root / relative
                directory.mkdir(parents=True)
                (directory / "Cargo.toml").write_text(f'[package]\nname = "package{ordinal}"\n[features]\nfull = []\nbase = []\n')
            target = root / "src/core/tests/fresh.rs"
            target.parent.mkdir()
            target.write_text('#[cfg(feature = "full")]\n#[test]\nfn actual() {}\n')
            owners = root / ".github/ci/owners.toml"
            owners.parent.mkdir(parents=True)
            owners.write_text("version = 1\n")
            action = root / ".github/actions/unused/action.yml"
            action.parent.mkdir(parents=True)
            action.write_text('''inputs:
  profile:
    default: full
runs:
  using: composite
  steps:
    - shell: bash
      run: cargo test -p package0 --features ${{ inputs.profile }} --test fresh
''')
            workflow = root / ".github/workflows/test.yml"
            workflow.parent.mkdir()
            for steps, owned in (
                ("  - run: true", False),
                ("  - uses: ./.github/actions/unused", True),
                ("  - uses: ./.github/actions/unused\n    with:\n      profile: full", True),
                ("  - uses: ./.github/actions/unused\n    with:\n      profile: base", False),
                ("  - uses: ./.github/actions/unused\n    with:\n      profile: 'full --no-run'", False),
                ("  - uses: ./.github/actions/unused\n    with:\n      profile: ${{ matrix.profile }}", False),
            ):
                workflow.write_text("steps:\n" + steps + "\n")
                with self.subTest(steps=steps):
                    self.assertEqual(inventory_gaps(root), [] if owned else ["package0:fresh (src/core/tests/fresh.rs)"])

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

    def test_custom_harness_targets_cannot_own_ordinary_registered_tests(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "tests").mkdir()
            (directory / "tests/custom.rs").write_text("fn main() {}\n#[test]\nfn panics() { panic!(); }\n")
            for harness in (False, True):
                manifest = {"test": [{"name": "custom", "harness": harness}]}
                target = integration_targets(directory, manifest)["custom"]
                self.assertIs(target["harness"], harness)
                self.assertEqual(target_covered("sample", "custom", target, manifest,
                                                [self.command(["--test", "custom"])]), harness)

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
