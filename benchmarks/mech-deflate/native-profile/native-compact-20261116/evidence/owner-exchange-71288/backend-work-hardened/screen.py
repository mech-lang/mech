"""Bind the compiler flags independently through the same fixed worker link."""

from hashlib import file_digest
from importlib import import_module
import json
import os
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
macho = import_module("reachability-census")
compiler = q / "host-opt-levels/opt3-cargo/release/compile_profile"
reference = q / "native-size-20261117/source-aligned"
packet = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
source = packet / "program.mec"
output = study / "worker-checks"
env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
runtime = json.loads((reference / "runtime.json").read_text())
baseline = runtime["flags"] | {"MECH_COMPACT_INLINE_SINGLE_CALLS": "preselected"}
unpadded = {key: value for key, value in baseline.items() if key != "MECH_COMPACT_ROOT_TAIL_PROFILE"}
assert len(unpadded) == len(baseline) - 1
refused = "MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA"
exchange = "MECH_COMPACT_SHARE_OWNER_EXCHANGE"
assert not set(baseline).intersection((refused, exchange))
assert not any("LLVM" in key for key in baseline)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def run(args, log, overrides=None):
    command = list(map(str, args))
    with log.open("wb") as out:
        subprocess.run(command, cwd=q, env=env | (overrides or {}), stdout=out,
                       stderr=subprocess.STDOUT, check=True)
    return command


def summarize(label, flags):
    out = output / label
    out.mkdir()
    command = run([compiler, source, out, "compact"], out / "compile.log", flags)
    metadata = json.loads((out / "metadata.json").read_text())
    names = ("program.mecb", "program.o", "metadata.json", "mech-worker")
    link = runtime["link"]
    for file in ("program.o", "mech-worker"):
        assert sum(arg.count(str(reference / file)) for arg in link) == 1
        link = [arg.replace(str(reference / file), str(out / file)) for arg in link]
    run(link, out / "link.log")
    hashes = {name: sha(out / name) for name in names}
    assert hashes["program.mecb"] == runtime["image_sha256"]
    write(out / "runtime.json", runtime | {"link": link, "emit": command, "flags": flags})
    smoke = f"hardened-backend-{label}"
    verified = run([sys.executable, "-B", q / "verify-worker.py", out / "mech-worker",
                    out / "metadata.json", smoke], out / "verify.log")
    proof = json.loads((q / f"{smoke}-qualification.json").read_text())
    assert proof["complete_smokes"] == 162 and proof["worker_sha256"] == hashes["mech-worker"]
    log = (out / "compile.log").read_text().splitlines()
    text = macho.load(out / "program.o")[1][macho.TEXT][2]
    compilation = {
        "encoder_text_bytes": text, "encoder_text_x_roc": text / 57696, "encoder_text_x_libdeflate": text / 37080,
        "object_bytes": (out / "program.o").stat().st_size,
        "linked_text_bytes": macho.load(out / "mech-worker")[1][macho.TEXT][2],
        "linked_file_bytes": (out / "mech-worker").stat().st_size,
        "root_code_bytes": metadata["code_bytes"], "root_frame_bytes": metadata["stack_bytes"],
        "skipped_ssa": [x for x in log if "skipped general numeric SSA" in x],
        "owner_exchanges": [x for x in log if "compact owner exchanges shared:" in x],
    }
    result = {"role": label, "flags": flags, "sha256": hashes, "metrics": compilation,
              "link": link, "compile": command, "smokes": proof | {"driver": verified}}
    write(out / "result.json", result)
    print(label, compilation, hashes["mech-worker"], flush=True)
    return result, metadata


assert sha(compiler) == json.loads((study / "compiler-build.json").read_text())["compiler_sha256"]
assert sha(source) == json.loads((packet / "recipe.json").read_text())["source_sha256"]
output.mkdir()
write(output / "plan.json", {"source_sha256": sha(source), "compiler_sha256": sha(compiler),
                             "source_and_patches": sha(study / "source.json"),
                             "scope": "Untimed artifact, size and correctness screen.",
                             "retained_flags": baseline, "owner_flags": unpadded})
control, prior = summarize("retained", baseline)
for name in ("program.o", "metadata.json", "mech-worker"):
    assert control["sha256"][name] == sha(reference / name), name
skip, skipped = summarize("refused-ssa", baseline | {refused: "1"})
assert skip["sha256"] == control["sha256"]
assert len(skip["metrics"]["skipped_ssa"]) == 1 and not skip["metrics"]["owner_exchanges"]
size_control, unpadded_metadata = summarize("size-control", unpadded)
trial, candidate = summarize("owner-exchange", unpadded | {exchange: "trace"})
assert trial["sha256"]["program.mecb"] == size_control["sha256"]["program.mecb"]
assert candidate["origins"] == unpadded_metadata["origins"] == prior["origins"] == skipped["origins"]
assert not trial["metrics"]["skipped_ssa"]
write(output / "variants.json", [control, skip, size_control, trial])
