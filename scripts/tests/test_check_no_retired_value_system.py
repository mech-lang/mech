from __future__ import annotations

import importlib.util
import tempfile
import unittest
import json
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-no-retired-value-system.py"
SPEC = importlib.util.spec_from_file_location("check_no_retired_value_system", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class RetiredValueSystemAbsenceTests(unittest.TestCase):
    def fixture(self, source: str = "pub struct Canonical;\n") -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        path = root / "src/core/src/lib.rs"
        path.parent.mkdir(parents=True)
        path.write_text(source, encoding="utf-8")
        surface = root / CHECKER.SURFACE_PATH.relative_to(CHECKER.ROOT)
        surface.parent.mkdir(parents=True, exist_ok=True)
        surface.write_text(CHECKER.SURFACE_PATH.read_text(encoding="utf-8"), encoding="utf-8")
        inventory = json.loads(surface.read_text(encoding="utf-8"))
        for declaration in inventory["retained_declarations"]:
            declaration_path = root / declaration["path"]
            declaration_path.parent.mkdir(parents=True, exist_ok=True)
            with declaration_path.open("a", encoding="utf-8") as fixture:
                fixture.write(f"\n{declaration['fixture']}\n")
        return root

    def test_canonical_tree_passes(self):
        self.assertEqual(CHECKER.failures(self.fixture()), [])

    def test_retired_symbol_fails(self):
        failures = CHECKER.failures(self.fixture("fn escape(_: LegacyValue) {}\n"))
        self.assertTrue(any("LegacyValue" in failure for failure in failures))

    def test_every_retired_symbol_is_rejected(self):
        for symbol in CHECKER.retired_surface(self.fixture())["retired_symbols"]:
            with self.subTest(symbol=symbol):
                failures = CHECKER.failures(
                    self.fixture(f"fn escape(_: {symbol}) {{}}\n")
                )
                self.assertTrue(any(symbol in failure for failure in failures))

    def test_every_retired_conversion_is_rejected(self):
        for conversion in CHECKER.retired_surface(self.fixture())["retired_conversions"]:
            with self.subTest(conversion=conversion):
                failures = CHECKER.failures(
                    self.fixture(f"fn escape() {{ {conversion}(); }}\n")
                )
                self.assertTrue(any(conversion in failure for failure in failures))

    def test_retired_module_declaration_fails(self):
        failures = CHECKER.failures(self.fixture("pub mod legacy_value;\n"))
        self.assertTrue(any("module declaration" in failure for failure in failures))

    def test_every_retired_module_visibility_and_body_form_fails(self):
        for declaration in (
            "mod legacy_adapter {}",
            "pub mod legacy_adapter {}",
            "pub(crate) mod legacy_adapter {}",
            "pub(super) mod legacy_adapter;",
        ):
            with self.subTest(declaration=declaration):
                failures = CHECKER.failures(self.fixture(declaration + "\n"))
                self.assertTrue(
                    any("module declaration" in failure for failure in failures)
                )

    def test_comments_and_string_literals_do_not_trigger_symbol_checks(self):
        root = self.fixture(
            '// LegacyValue and pub mod legacy_adapter {}\n'
            'const MESSAGE: &str = "LegacyValue pub mod legacy_adapter {}";\n'
            'const RAW: &str = r#"ValueKind MutableReference"#;\n'
            '/* nested /* KindTable */ LegacyValue */\n'
        )
        self.assertEqual(CHECKER.failures(root), [])

    def test_character_literals_do_not_hide_following_declarations(self):
        for literal in (
            "'\"'",
            "b'\"'",
            "'\\''",
            "b'\\''",
            "'\\u{2764}'",
            "b'\\x7f'",
        ):
            with self.subTest(literal=literal):
                failures = CHECKER.failures(
                    self.fixture(
                        f"const QUOTE: char = {literal};\n"
                        "pub mod legacy_adapter {}\n"
                    )
                )
                self.assertTrue(
                    any("module declaration" in failure for failure in failures)
                )

    def test_character_and_byte_character_contents_do_not_trigger_checks(self):
        root = self.fixture(
            "const KIND: char = 'K';\n"
            "const BYTE: u8 = b'V';\n"
            "fn lifetime<'a>(value: &'a str) -> &'a str { value }\n"
        )
        self.assertEqual(CHECKER.failures(root), [])

    def test_lifetimes_do_not_mask_retired_declarations(self):
        failures = CHECKER.failures(
            self.fixture("fn borrow<'a>(_: &'a str) {} pub enum Kind { Any }\n")
        )
        self.assertTrue(any("retired semantic Kind enum" in failure for failure in failures))

    def test_raw_identifier_retired_declarations_fail(self):
        for declaration in (
            "mod r#legacy_adapter {}",
            "pub(crate) mod r#legacy_value;",
            "pub enum r#Kind { Any }",
        ):
            with self.subTest(declaration=declaration):
                failures = CHECKER.failures(self.fixture(declaration + "\n"))
                self.assertTrue(failures)

    def test_non_rust_include_targets_are_scanned_as_executable_rust(self):
        root = self.fixture('include!("retired.inc");\n')
        included = root / "src/core/src/retired.inc"
        included.write_text("pub mod legacy_adapter {}\n", encoding="utf-8")
        failures = CHECKER.failures(root)
        self.assertTrue(any("retired.inc" in failure for failure in failures))

    def test_dynamic_include_targets_fail_closed(self):
        failures = CHECKER.failures(
            self.fixture('include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n')
        )
        self.assertTrue(any("not a static path" in failure for failure in failures))

    def test_every_retained_canonical_declaration_is_required(self):
        surface = CHECKER.retired_surface(self.fixture())
        for declaration in surface["retained_declarations"]:
            with self.subTest(symbol=declaration["symbol"]):
                root = self.fixture()
                path = root / declaration["path"]
                path.write_text("pub struct Missing;\n", encoding="utf-8")
                failures = CHECKER.failures(root)
                self.assertTrue(
                    any(
                        declaration["symbol"] in failure
                        and "declaration is missing" in failure
                        for failure in failures
                    )
                )

    def test_retired_path_fails(self):
        surface = CHECKER.retired_surface(self.fixture())
        for relative in surface["forbidden_paths"]:
            with self.subTest(relative=relative):
                root = self.fixture()
                path = root / relative
                if Path(relative).suffix:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text("retired\n", encoding="utf-8")
                else:
                    path.mkdir(parents=True)
                self.assertTrue(
                    any("retired path" in failure for failure in CHECKER.failures(root))
                )

    def test_retired_surface_covers_reviewed_public_escape_hatches(self):
        surface = CHECKER.retired_surface(self.fixture())
        self.assertTrue(
            {"AsValueKind", "ToValue", "LegacyReactivePlanRegistration"}
            <= set(surface["retired_symbols"])
        )

    def test_surface_inventory_records_every_deleted_public_declaration_class(self):
        surface = CHECKER.retired_surface(self.fixture())
        self.assertEqual(surface["source_base_sha"], "fe2e71425c78cae913ef3b01f622f72bceb7438c")
        self.assertEqual(len(surface["source_paths"]), 20)
        self.assertEqual(len(surface["forbidden_paths"]), 8)
        self.assertEqual(len(surface["retired_symbols"]), 73)
        self.assertEqual(len(surface["retired_conversions"]), 41)
        self.assertEqual(len(surface["retired_declarations"]), 1)
        self.assertEqual(len(surface["retained_declarations"]), 7)
        self.assertEqual(surface["path_guarded_ambiguous_symbols"], ["decode"])
        self.assertTrue(
            {
                "CompileConst",
                "ToIndex",
                "canonical_bytecode_composite_children",
            }
            <= set(surface["retained_canonical_symbols"])
        )
        self.assertTrue(
            {"MechSet", "UnhandledFunctionArgumentIxesMono"}
            <= set(surface["retired_symbols"])
        )
        self.assertTrue(
            set(surface["retired_symbols"]).isdisjoint(
                surface["retained_canonical_symbols"]
            )
        )
        self.assertTrue(
            {"snapshot_from_legacy", "function_invocation_from_legacy"}
            <= set(surface["retired_conversions"])
        )
        self.assertTrue(
            {
                "src/core/src/legacy_value.rs",
                "src/core/src/legacy_adapter.rs",
            }
            <= set(surface["forbidden_paths"])
        )

    def test_retired_semantic_kind_declaration_fails(self):
        failures = CHECKER.failures(self.fixture("pub enum Kind { Any }\n"))
        self.assertTrue(any("retired semantic Kind enum" in failure for failure in failures))

    def test_parser_kind_declaration_remains_allowed(self):
        root = self.fixture()
        parser = root / "src/core/src/nodes.rs"
        parser.write_text("pub enum Kind { Scalar }\n", encoding="utf-8")
        self.assertEqual(CHECKER.failures(root), [])


if __name__ == "__main__":
    unittest.main()
