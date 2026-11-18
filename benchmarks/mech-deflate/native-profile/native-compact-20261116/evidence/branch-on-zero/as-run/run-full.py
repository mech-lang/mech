"""Run both full orders with the exact aligned worker and recorded run bindings."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import sys

study = Path(__file__).resolve().parent
q = study.parent
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(q))


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(name, value):
    path = study / name
    assert not path.exists()
    path.write_text(json.dumps(value, indent=2) + "\n")
    return digest(path)


prepared = study / "aligned-timing-plan.json"
assert digest(prepared) == (study / "aligned-timing-plan.sha256").read_text().strip()
plan = json.loads(prepared.read_text())
environment = write("aligned-environment.json", {
    "cpu": plan["measurement_host"], "os": plan["platform"], "clock": "Darwin Rust Instant",
    "conditions": "Serial, same pinned runtime, comparator binaries and adapters.",
})
contract = write("aligned-contract.json", {
    "scope": "diagnostic", "level": 12, "timing_boundary": plan["contract"],
    "fixture_inventory_sha256": digest(packet / "fixtures.json"),
})
plan_sha = write("aligned-full-bindings.json", {
    "original_plan_sha256": digest(prepared), "environment_sha256": environment,
    "contract_sha256": contract, "runs": [row for row in plan["runs"] if row["kind"] == "full"],
})
(study / "aligned-full-bindings.sha256").write_text(plan_sha + "\n")
os.environ.update(ZERO_BRANCH_ENVIRONMENT_SHA256=environment, ZERO_BRANCH_CONTRACT_SHA256=contract)
driver = q / "benchmark-matrix-size-variants.py"
code = driver.read_text()
assert digest(driver) == plan["driver_sha256"] and digest(q / "compare-library.py") == plan["timer_sha256"]
read_source = "source = driver.read_text()"
hook = "assert digest(FIXTURES) == pins['input_inventory_sha256']"
assert code.count(read_source) == 1
binding = hook + "\n" + (
    'pins["environment_sha256"] = os.environ["ZERO_BRANCH_ENVIRONMENT_SHA256"]\n'
    'pins["contract_sha256"] = os.environ["ZERO_BRANCH_CONTRACT_SHA256"]'
)
code = code.replace(read_source, f"{read_source}\nassert source.count({hook!r}) == 1\n"
                                f"source = source.replace({hook!r}, {binding!r})")
sys.argv = [
    str(driver), "branch-on-zero-aligned", "--reference", "cranelift-width-16-owners-cold",
    "--reuse-focus", "--full-diagnostic", "--case-set", str(study / "zero-branch-cases.json"),
]
exec(compile(code, str(driver), "exec"), {"__file__": str(driver), "__name__": "__main__"})
