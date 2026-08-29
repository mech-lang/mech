import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-production-legacy-closure.py"
SPEC = importlib.util.spec_from_file_location("production_legacy_closure", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class ProductionLegacyClosureTests(unittest.TestCase):
    def test_comments_and_literals_do_not_count_as_production_references(self):
        source = '''
// LegacyValue is historical prose.
const NOTE: &str = "ValueKind is historical prose";
/* MutableReference is historical prose. */
pub fn canonical() {}
'''

        searchable = CHECKER.mask_comments_and_literals(source)

        for _, pattern in CHECKER.PROHIBITED:
            self.assertIsNone(pattern.search(searchable))

    def test_cfg_test_modules_are_explicit_compatibility_tests(self):
        source = '''
#[cfg(test)]
mod compatibility_tests {
    use crate::{FunctionArgs, LegacyValue};
    fn legacy_fixture() {
        let _ = FunctionArgs::Nullary(LegacyValue::Empty);
    }
}
pub fn canonical() {}
'''

        searchable = CHECKER.mask_comments_and_literals(
            CHECKER.mask_test_modules(source)
        )

        for _, pattern in CHECKER.PROHIBITED:
            self.assertIsNone(pattern.search(searchable))

    def test_production_references_remain_visible(self):
        source = "pub fn leak(value: LegacyValue) { let _ = FunctionArgs::Nullary(value); }"
        searchable = CHECKER.mask_comments_and_literals(
            CHECKER.mask_test_modules(source)
        )
        found = {
            label
            for label, pattern in CHECKER.PROHIBITED
            if pattern.search(searchable)
        }

        self.assertEqual(found, {"LegacyValue", "FunctionArgs construction"})

    def test_legacy_aggregate_function_port_backings_are_rejected(self):
        source = """
impl function_port_backing::Sealed for crate::MechSet {}
impl<T: FunctionPortBacking> function_port_backing::Sealed for Matrix<T> {}
impl function_state_sealed::PortSealed for crate::MechMap {
    type ElementShape = ();
}
"""
        searchable = CHECKER.mask_comments_and_literals(
            CHECKER.mask_test_modules(source)
        )
        found = {
            label
            for label, pattern in CHECKER.LEGACY_AGGREGATE_PORT_BACKINGS
            if pattern.search(searchable)
        }

        self.assertEqual(
            found,
            {
                "legacy aggregate function-port backing",
                "legacy aggregate state-port backing",
            },
        )

    def test_exact_matrix_backings_remain_allowed(self):
        source = """
impl<T> function_port_backing::Sealed for crate::DMatrix<T> {}
impl<T> function_state_sealed::PortSealed for crate::Matrix2<T> {}
"""
        for _, pattern in CHECKER.LEGACY_AGGREGATE_PORT_BACKINGS:
            self.assertIsNone(pattern.search(source))


if __name__ == "__main__":
    unittest.main()
