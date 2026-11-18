"""Run the original matched driver with the predeclared campaign bindings."""

import json
from pathlib import Path
import os
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
plan = json.loads((study / "plan.json").read_text())
for field in ("environment_sha256", "contract_sha256"):
    os.environ[f"ALIGNED_SPILLS_{field}"] = plan[field]
driver = q / "benchmark-matrix-size-variants.py"
code = driver.read_text()
old = "source = driver.read_text()"
assert code.count(old) == 1
hook = 'assert digest(FIXTURES) == pins[\'input_inventory_sha256\']'
binding = '\n'.join(
    f'pins[{field!r}] = os.environ["ALIGNED_SPILLS_{field}"]'
    for field in ("environment_sha256", "contract_sha256")
)
code = code.replace(old, f"{old}\nassert source.count({hook!r}) == 1\n"
                   f"source = source.replace({hook!r}, {hook + chr(10) + binding!r})")
sys.argv[0] = str(driver)
exec(compile(code, str(driver), "exec"), {"__file__": str(driver), "__name__": "__main__"})
