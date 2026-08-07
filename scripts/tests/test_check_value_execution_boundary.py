import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-value-execution-boundary.py"
SPEC = importlib.util.spec_from_file_location("value_execution_boundary", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class ValueExecutionBoundaryTests(unittest.TestCase):
    def fixture(self, sources, allowed=None, pattern="legacy_call()"):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative, source in sources.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source, encoding="utf-8")
        manifest = root / "manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "reference_commit": "fixture",
                    "boundaries": [
                        {
                            "id": "legacy",
                            "description": "fixture boundary",
                            "pattern": pattern,
                            "allowed": allowed if allowed is not None else [
                                {
                                    "path": "src/pkg/src/good.rs",
                                    "scope_contains": "fn approved",
                                    "max_occurrences": 1,
                                }
                            ],
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )
        return temporary, root, manifest

    def test_empty_allowed_passes_when_pattern_is_absent(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/good.rs": "fn production() {}"}, allowed=[]
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_empty_allowed_rejects_occurrence_anywhere(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/new.rs": "fn production() { legacy_call(); }"},
            allowed=[],
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("legacy: unapproved occurrence", failures[0])

    def test_approved_occurrence_passes(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/good.rs": "fn approved() { legacy_call(); }"}
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_deleted_occurrence_passes(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/good.rs": "fn approved() {}"}
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_new_path_fails_with_location(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}",
                "src/pkg/src/new.rs": "fn added() { legacy_call(); }",
            }
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("legacy: unapproved occurrence", failures[0])
        self.assertIn("src/pkg/src/new.rs:1", failures[0])

    def test_unapproved_value_state_journal_use_fails(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/new.rs": (
                    "fn added(journal: &ValueStateJournal) { let _ = journal; }"
                )
            },
            allowed=[],
            pattern="ValueStateJournal",
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("legacy: unapproved occurrence", failures[0])
        self.assertIn("src/pkg/src/new.rs:1", failures[0])

    def test_unapproved_reactive_cell_id_use_fails(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/new.rs": "fn added(cell: ReactiveCellId) { let _ = cell; }"},
            allowed=[],
            pattern="ReactiveCellId",
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("legacy: unapproved occurrence", failures[0])
        self.assertIn("src/pkg/src/new.rs:1", failures[0])

    def test_excess_occurrence_fails(self):
        temporary, root, manifest = self.fixture(
            {"src/pkg/src/good.rs": "fn approved() { legacy_call(); legacy_call(); }"}
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertTrue(any("maximum is 1" in failure for failure in failures))

    def test_test_only_directory_is_ignored(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}",
                "src/pkg/src/tests/snippet.rs": "fn test_only() { legacy_call(); }",
            }
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_path_based_test_module_file_is_ignored(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}",
                "src/pkg/src/snapshot/tests.rs": "fn regression() { legacy_call(); }",
            }
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_inline_cfg_test_module_is_ignored(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/inline.rs": (
                    "#[cfg(test)]\n"
                    "mod tests {\n"
                    "    const BRACES: &str = \"{ still test-only }\";\n"
                    "    fn rejected_text_fixture() { legacy_call(); }\n"
                    "}\n"
                    "fn production() {}\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_direct_cfg_test_function_and_impl_are_ignored(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/direct.rs": (
                    "#[cfg(test)]\n"
                    "#[test]\n"
                    "fn regression() { legacy_call(); }\n"
                    "#[cfg(all(test, feature = \"fixture\"))]\n"
                    "impl Fixture { fn regression(&self) { legacy_call(); } }\n"
                    "fn production() {}\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        self.assertEqual(CHECKER.audit(root, manifest), [])

    def test_direct_cfg_any_test_or_feature_function_is_audited(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/direct.rs": (
                    "#[cfg(any(test, feature = \"runtime\"))]\n"
                    "fn production() { legacy_call(); }\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("src/pkg/src/direct.rs:2", failures[0])

    def test_cfg_test_mask_preserves_production_locations(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/inline.rs": (
                    "#[cfg(all(test, feature = \"fixture\"))]\n"
                    "mod tests { fn ignored() { legacy_call(); } }\n"
                    "fn production() { legacy_call(); }\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("src/pkg/src/inline.rs:3", failures[0])

    def test_cfg_not_test_module_is_audited(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/inline.rs": (
                    "#[cfg(not(test))]\n"
                    "mod production { fn dependency() { legacy_call(); } }\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("src/pkg/src/inline.rs:2", failures[0])

    def test_cfg_any_test_or_feature_module_is_audited(self):
        temporary, root, manifest = self.fixture(
            {
                "src/pkg/src/good.rs": "fn approved() {}\n",
                "src/pkg/src/inline.rs": (
                    "#[cfg(any(test, feature = \"runtime\"))]\n"
                    "mod production { fn dependency() { legacy_call(); } }\n"
                ),
            }
        )
        self.addCleanup(temporary.cleanup)
        failures = CHECKER.audit(root, manifest)
        self.assertEqual(len(failures), 1)
        self.assertIn("src/pkg/src/inline.rs:2", failures[0])


if __name__ == "__main__":
    unittest.main()
