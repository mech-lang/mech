"""Measure encoder text under explicit continuation argument budgets."""

import json
from pathlib import Path
import subprocess
import sys

q = Path(__file__).resolve().parent
out = q / "retained-61-continuation-widths"
out.mkdir()
reference = q / "cranelift-continuation-registers-word-pages-native/final-candidate"
emitter = q / "native-review-pr/baseline-build/compiler/release/emit_compact_size"
image = q / "shared-huffman-packed-writer-shell-sort-keys-reachable-unmapped-native/frontend/program.mecb"
size_denominators = {"roc": 57696, "libdeflate": 37080}
direct = "MECH_COMPACT_DIRECT_CONTINUATION_WORDS"
owners = "MECH_COMPACT_FOLD_FRESH_OWNER_COUNTS"
cold = "MECH_COMPACT_SHARE_COLD_TAILS"

cases = []


def screen(name, **overrides):
    directory = out / name
    command = [sys.executable, "-B", q / "probe-literal-reuse.py", "--native-reference", reference,
               "--emitter", emitter, "--emit-only", directory, image]
    command.extend(f"{key}={value}" for key, value in overrides.items())
    subprocess.run(command, check=True)
    receipt = json.loads((directory / "receipt.json").read_text())
    size = receipt["object_text_bytes"]
    receipt.update(roc_object_text_bytes=size_denominators["roc"],
                   libdeflate_object_text_bytes=size_denominators["libdeflate"],
                   object_text_x_roc=size / size_denominators["roc"],
                   object_text_x_libdeflate=size / size_denominators["libdeflate"])
    (directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    cases.append({"name": name, "flag_overrides": overrides, **receipt})
    print(name, size, receipt["root_frame_bytes"], flush=True)


for width in (4, 6, 8, 10, 12, 14):
    screen(f"width-{width}", **{direct: str(width)})
assert cases[1]["object_sha256"] == "ddf45b958b4256cd7b64f41af0b964cfdfa0f7747ed3f4a13070e2fb53ec48f9"
smallest = min(cases, key=lambda row: row["object_text_bytes"])
width = smallest["flag_overrides"][direct]
screen("width-16-owners-cold", **{owners: "1", cold: "1"})
screen(f"width-{width}-owners", **{direct: width, owners: "1"})
screen(f"width-{width}-owners-cold", **{direct: width, owners: "1", cold: "1"})

report = {"scope": "Static size and root frame screen on the pinned bytecode, with the retained native recipe plus the listed overrides.",
          "runtime_qualification": "These emissions are unlinked and have no new per-case timings.",
          "size_denominators": size_denominators, "cases": cases}
(out / "screen.json").write_text(json.dumps(report, indent=2) + "\n")
