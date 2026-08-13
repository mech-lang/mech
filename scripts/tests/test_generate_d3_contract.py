from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_d3_contract", ROOT / "scripts/generate-d3-contract.py"
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class D3ContractGeneratorTests(unittest.TestCase):
    def test_repository_contains_all_projections_and_schemas(self):
        generated = GENERATOR.generated_files()
        self.assertEqual(len(generated), 12)
        for path, content in generated.items():
            self.assertTrue(path.exists())
            self.assertEqual(path.read_bytes(), content)
            json.loads(content)

    def test_profile_freezes_the_external_boundary(self):
        values = GENERATOR.values()
        self.assertEqual(values["profile"]["bytecode_format"], "v1")
        self.assertFalse(values["profile"]["normal_product_routing_changed"])
        self.assertEqual(values["replay"]["provider_reads"], 0)
        self.assertEqual(values["failure-matrix"]["rejected_publication_stores"], 0)
        self.assertEqual(len(values["failure-matrix"]["cases"]), 26)


if __name__ == "__main__":
    unittest.main()
