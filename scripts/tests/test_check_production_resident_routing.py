import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "production_resident_routing",
    Path(__file__).resolve().parents[1] / "check-production-resident-routing.py",
)
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class CliFactorySeamTests(unittest.TestCase):
    @staticmethod
    def source(body):
        return "pub fn register_cli_host_factories(builder: RuntimeBuilder) -> MResult<()> {\n" + body + "\n}"

    def test_explicit_empty_snapshot_reaches_registration(self):
        for prefix in ("", "mech_terminal::"):
            source = self.source(
                f"let cli_factory = {prefix}CliHostFactory::with_arguments(Vec::new())?;\n"
                "register_cli_host_factories_with_cli_factory(builder, Box::new(cli_factory))"
            )
            self.assertTrue(CHECK.explicit_empty_cli_factory_is_registered(source))

    def test_ambient_nonempty_or_unregistered_factories_are_rejected(self):
        for body in (
            "let cli_factory = CliHostFactory::new()?;\nregister_cli_host_factories_with_cli_factory(builder, Box::new(cli_factory))",
            "let cli_factory = CliHostFactory::with_arguments(std::env::args_os().collect())?;\nregister_cli_host_factories_with_cli_factory(builder, Box::new(cli_factory))",
            'let cli_factory = CliHostFactory::with_arguments(vec!["leaked".into()])?;\nregister_cli_host_factories_with_cli_factory(builder, Box::new(cli_factory))',
            "let cli_factory = CliHostFactory::with_arguments(Vec::new())?;\nregister_cli_host_factories_with_cli_factory(builder, Box::new(other))",
            "let cli_factory = CliHostFactory::with_arguments(Vec::new())?;\nOk(())",
        ):
            with self.subTest(body=body):
                self.assertFalse(CHECK.explicit_empty_cli_factory_is_registered(self.source(body)))

    def test_comments_literals_and_disabled_branches_cannot_claim_registration(self):
        real = "let cli_factory = CliHostFactory::with_arguments(Vec::new())?;\nregister_cli_host_factories_with_cli_factory(builder, Box::new(cli_factory))"
        for body in (
            "// " + real.replace("\n", "\n// ") + "\nOk(())",
            "/* " + real + " */\nOk(())",
            'let _note = r#"' + real + '"#;\nOk(())',
            "if false { " + real + " }\nOk(())",
        ):
            with self.subTest(body=body):
                self.assertFalse(CHECK.explicit_empty_cli_factory_is_registered(self.source(body)))


class BrowserProjectSeamTests(unittest.TestCase):
    def executor_fixture(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        engine_lib = root / "src/engine/src/lib.rs"
        engine_lib.parent.mkdir(parents=True)
        engine_lib.write_text("pub mod resident;\n", encoding="utf-8")
        return root

    def test_shipping_seams_pass(self):
        self.assertEqual(CHECK.check_required_product_seams(), [])

    def test_losing_interactive_root_symbols_is_rejected(self):
        read_text = Path.read_text

        def without_interactive_root(path, *args, **kwargs):
            source = read_text(path, *args, **kwargs)
            if path == CHECK.ROOT / "src/wasm/src/project.rs":
                body = CHECK.rust_function_body(source, "run_source_roots")
                self.assertIsNotNone(body)
                source = source.replace(
                    body,
                    body.replace("load_interactive_root_program", "load_root_program"),
                    1,
                )
            return source

        with patch.object(Path, "read_text", without_interactive_root):
            failures = CHECK.check_required_product_seams()
        self.assertIn(
            "src/wasm/src/project.rs: run_source_roots must load the interactive resident root program",
            failures,
        )

    def test_retired_executor_absence_passes(self):
        self.assertEqual(CHECK.check_retired_executor_boundary(self.executor_fixture()), [])

    def test_disabled_private_interpreter_restoration_is_rejected(self):
        for declaration in (
            '#[cfg(any())]\nmod interpreter;\n',
            '#[cfg(any())]\npub use self::interpreter::*;\n',
        ):
            with self.subTest(declaration=declaration):
                root = self.executor_fixture()
                (root / "src/engine/src/lib.rs").write_text(declaration, encoding="utf-8")
                self.assertIn(
                    "src/engine/src/lib.rs: retired interpreter module must not remain reachable",
                    CHECK.check_retired_executor_boundary(root),
                )

    def test_unexported_executor_workspace_restoration_is_rejected(self):
        root = self.executor_fixture()
        executor = root / "src/engine/src/interpreter/mod.rs"
        executor.parent.mkdir()
        executor.write_text("// retired even without a module declaration\n", encoding="utf-8")
        self.assertIn(
            "src/engine/src/interpreter: retired executor workspace remains",
            CHECK.check_retired_executor_boundary(root),
        )

    def test_mutable_instance_restoration_is_rejected(self):
        root = self.executor_fixture()
        instance = root / "src/engine/src/program/instance.rs"
        instance.parent.mkdir()
        instance.write_text("// retired mutable instance\n", encoding="utf-8")
        self.assertIn(
            "src/engine/src/program/instance.rs: obsolete program instance remains",
            CHECK.check_retired_executor_boundary(root),
        )

    def test_shipping_old_executor_call_restoration_is_rejected(self):
        with patch.object(
            CHECK,
            "product_sources",
            return_value=[(Path("src/cli/run.rs"), "fn ship(engine: &mut X) { engine.run_tree(tree); }")],
        ):
            failures = CHECK.check_product_references()
        self.assertTrue(any("direct old executor call .run_tree(" in row for row in failures))


if __name__ == "__main__":
    unittest.main()
