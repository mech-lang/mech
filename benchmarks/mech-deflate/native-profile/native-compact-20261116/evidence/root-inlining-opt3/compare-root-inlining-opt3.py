"""Measure the root recompile ablation using the optimized compiler and fixed link."""

from importlib import import_module
import json
from pathlib import Path
from statistics import median
import subprocess
import sys

measure = import_module("compare-builds")
q = measure.q
study = q / "host-opt-levels"
output = study / "root-inlining"
measure.source_mech = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec"
measure.frontend = study / "opt3-cargo/release/native_compact_frontend"
measure.emitter = study / "opt3-cargo/release/emit_compact_size"

pinned = study / "pinned-opt3-default"
if len(sys.argv) != 2:
    raise SystemExit("expected the checked no-inline encoder directory")
no_inline = Path(sys.argv[1]).resolve()
modes = {
    name: tuple(json.loads((path / file).read_text()) for file in ("runtime.json", "receipt.json"))
    for name, path in (("retained", pinned), ("no-inline", no_inline))
}
baseline, original = modes["retained"]
ablation, candidate = modes["no-inline"]
expected_removed = {"MECH_COMPACT_INLINE_SINGLE_CALLS", "MECH_COMPACT_INLINE_SYMBOLS"}
assert ablation["flags"] == {key: value for key, value in baseline["flags"].items() if key not in expected_removed}
assert ablation["image_sha256"] == baseline["image_sha256"]
library, = (arg.removeprefix("mech_gpu=") for arg in baseline["link"] if arg.startswith("mech_gpu="))
measure.mech_link = [library if path.endswith("/libmech_gpu_runtime.rlib") else path for path in measure.mech_link]


def run(name, directory, profile=False):
    measure.runtime, measure.reference_pins = modes[name]
    result = measure.measure("mech", directory, profile=profile)
    result["actor"] = name
    result["commands"] = json.loads((directory / "commands.json").read_text())
    return result


output.mkdir()
warmup = [run(name, output / "warmup" / name) for name in modes]
orders = [("retained", "no-inline"), ("no-inline", "retained")] * 4
trials = [run(name, output / f"order-{index}" / name)
          for index, order in enumerate(orders, 1) for name in order]
diagnostics = [run(name, output / "profile" / name, profile=True) for name in modes]
for result in (*trials, *diagnostics):
    reference = next(row for row in warmup if row["actor"] == result["actor"])
    assert result["executable_sha256"] == reference["executable_sha256"]
for check in warmup:
    executable = Path(check["executable"])
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", executable,
                    executable.parent / "native/metadata.json", f"root-inlining-opt3-{check['actor']}"], check=True)
medians = {
    name: {key: median(row["seconds"][key] for row in trials if row["actor"] == name)
           for key in warmup[0]["seconds"]}
    for name in modes
}
record = {
    "source_sha256": measure.sha(measure.source_mech),
    "compiled_with_opt_level": 3,
    "removed_flags": sorted(expected_removed),
    "orders": orders,
    "warmup": warmup,
    "trials": trials,
    "diagnostics": diagnostics,
    "medians": medians,
    "retained_encoder": original,
    "no_inline_encoder": candidate,
    "runtime_scope": "Exact output and alias checks. The changed encoder needs its own timing and stack qualification.",
}
(output / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(medians, indent=2), flush=True)
