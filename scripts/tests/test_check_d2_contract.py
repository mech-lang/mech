from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_d2_contract", ROOT / "scripts/check-d2-contract.py"
)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


class D2ContractTests(unittest.TestCase):
    def projections(self):
        projections, errors = CHECKER.load(ROOT)
        self.assertEqual(errors, [])
        return projections

    def test_repository_contract_passes(self):
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_projection_rejects_eager_or_opaque_substitutes(self):
        for projection, field, value in (
            ("artifact", "legacy_opaque_contracts", 1),
            ("artifact", "unclassified_nodes", 1),
            ("execution", "dirty_propagation", False),
            ("layout", "fixed_width_node_mask", True),
        ):
            projections = copy.deepcopy(self.projections())
            projections[projection][field] = value
            failures = CHECKER.projection_errors(projections)
            self.assertTrue(any(field in failure for failure in failures))

    def test_source_contract_has_real_dirty_propagation(self):
        self.assertEqual(CHECKER.source_errors(ROOT), [])


if __name__ == "__main__":
    unittest.main()
