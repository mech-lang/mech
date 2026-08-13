from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_d4_contract", ROOT / "scripts/check-d4-contract.py"
)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(CHECKER)


class D4ContractTests(unittest.TestCase):
    def test_repository_contract_passes_without_commit_topology(self):
        self.assertEqual(CHECKER.run(ROOT), [])

    def test_exact_nbody_transform_is_enforced(self):
        self.assertEqual(CHECKER.exact_nbody_errors(ROOT), [])

    def test_production_path_excludes_legacy_transaction_wrappers(self):
        sources = "\n".join(
            path.read_text()
            for path in (ROOT / "src/runtime/src/runtime/resident_program").glob("*.rs")
            if path.name != "tests.rs"
        )
        self.assertNotIn("RuntimeExecutionTransaction", sources)
        self.assertNotIn("commit_runtime", sources)
        self.assertNotIn("__resident", sources)


if __name__ == "__main__":
    unittest.main()
