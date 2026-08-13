from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_d3_contract", ROOT / "scripts/check-d3-contract.py"
)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(CHECKER)


class D3ContractTests(unittest.TestCase):
    def test_repository_contract_passes(self):
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_checker_enforces_forbidden_legacy_runtime_wrappers(self):
        coordinator = (ROOT / "src/runtime/src/resident_external/coordinator.rs").read_text()
        self.assertNotIn("RuntimeExecutionTransaction", coordinator)
        self.assertNotRegex(coordinator, r"\bcommit_runtime\s*\(")

    def test_checker_enforces_engine_value_boundary(self):
        resident = "\n".join(
            path.read_text() for path in (ROOT / "src/engine/src/resident").rglob("*.rs")
        )
        self.assertNotIn("LegacyValue", resident)
        self.assertNotIn("ValRef", resident)


if __name__ == "__main__":
    unittest.main()
