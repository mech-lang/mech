from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_d4_contract", ROOT / "scripts/generate-d4-contract.py"
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class D4ContractGeneratorTests(unittest.TestCase):
    def test_repository_contains_all_projections_and_schemas(self):
        generated = GENERATOR.generated_files()
        self.assertEqual(len(generated), 14)
        for path, content in generated.items():
            self.assertTrue(path.exists())
            self.assertEqual(path.read_bytes(), content)
            json.loads(content)

    def test_generator_is_identical_across_five_fresh_processes(self):
        snapshots = []
        script = (
            "import importlib.util, pathlib; "
            f"p=pathlib.Path({str(ROOT / 'scripts/generate-d4-contract.py')!r}); "
            "s=importlib.util.spec_from_file_location('d4',p); "
            "m=importlib.util.module_from_spec(s); s.loader.exec_module(m); "
            "print(''.join(x.hex() for _,x in sorted(m.generated_files().items())))"
        )
        for _ in range(5):
            snapshots.append(
                subprocess.run(
                    [sys.executable, "-B", "-c", script],
                    cwd=ROOT,
                    check=True,
                    text=True,
                    capture_output=True,
                ).stdout
            )
        self.assertEqual(len(set(snapshots)), 1)

    def test_profile_freezes_product_routing_and_nbody(self):
        values = GENERATOR.values()
        self.assertTrue(values["profile"]["normal_product_routing_changed"])
        self.assertEqual(values["profile"]["bytecode_format"], "v1")
        self.assertEqual(values["nbody"]["accepted_turns"], 4096)
        self.assertEqual(values["nbody"]["policy"], "RequireResident")
        self.assertEqual(values["hosts"]["scene"]["delivery"], "AtMostOnce")


if __name__ == "__main__":
    unittest.main()
