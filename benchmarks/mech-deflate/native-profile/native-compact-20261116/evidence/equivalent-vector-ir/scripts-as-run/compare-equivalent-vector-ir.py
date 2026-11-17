"""Measure equivalent-vector reuse with one compiler binary and fixed links."""

from collections import Counter
from importlib import import_module
import json
from pathlib import Path
from statistics import median
import subprocess
import sys

measure = import_module("compare-builds")
q = measure.q
study = q / "equivalent-vector-phases"
output = study / "matched"
packet = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
measure.source_mech = packet / "program.mec"
recipe = json.loads((packet / "recipe.json").read_text())
measure.frontend = study / "native_compact_frontend"
measure.emitter = q / "host-opt-levels/opt3-cargo/release/emit_compact_size"
assert measure.sha(measure.source_mech) == recipe["source_sha256"]
assert measure.sha(measure.roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
assert measure.sha(measure.source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"
flag = "MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR"
variants = {}
for case, name in (("retained", "cranelift-continuation-registers-word-pages"),
                   ("smaller", "cranelift-width-16-owners-cold")):
    reference = q / f"{name}-native/final-candidate"
    pins = json.loads((reference / "receipt.json").read_text())
    runtime = json.loads((reference / "runtime.json").read_text())
    assert runtime["image_sha256"] == recipe["bytecode_sha256"]
    assert flag not in runtime["flags"] and not any("LLVM" in key for key in runtime["flags"])
    variants[case] = runtime, pins
libraries = {arg.removeprefix("mech_gpu=") for runtime, _ in variants.values()
             for arg in runtime["link"] if arg.startswith("mech_gpu=")}
library, = libraries
measure.mech_link = [library if path.endswith("/libmech_gpu_runtime.rlib") else path for path in measure.mech_link]

mech_actors = {f"mech-{case}-{mode}": (case, mode) for case in variants for mode in ("control", "reuse")}
actors = (*mech_actors, "roc", "libdeflate")
pattern = (0, 1, 5, 2, 4, 3)
orders = [[actors[(shift + i) % len(actors)] for i in pattern] for shift in range(len(actors))]
assert all(set(position) == set(actors) for position in zip(*orders))
assert len(Counter((a, b) for order in orders for a, b in zip(order, order[1:]))) == 30


def runtime_for(actor, profile=False):
    case, mode = mech_actors[actor]
    runtime, pins = variants[case]
    flags = dict(runtime["flags"])
    if mode == "reuse":
        flags[flag] = "1"
    if profile:
        flags["MECH_PROFILE_COMPILE_JOBS"] = "1"
    return runtime | {"flags": flags}, pins


def native(actor, directory, profile=False):
    directory.mkdir(parents=True)
    runtime, pins = runtime_for(actor, profile)
    image = packet / "program.mecb"
    assert measure.sha(image) == runtime["image_sha256"]
    argv = [measure.emitter, image, directory]
    seconds = measure.run(argv, directory / "native.log", measure.env | runtime["flags"], instrument=profile)
    assert measure.sha(directory / "program.o") == pins["object_sha256"]
    assert measure.sha(directory / "metadata.json") == pins["metadata_sha256"]
    lines = (directory / "native.log").read_text().splitlines()
    reused = [line.partition(": ")[2] for line in lines if line.startswith("compact vector code reused:")]
    mode = mech_actors[actor][1]
    assert bool(reused) == (mode == "reuse"), (actor, reused)
    result = {"actor": actor, "seconds": seconds, "command": list(map(str, argv)),
              "effective_flags": runtime["flags"], "reused": reused,
              "jobs": [line for line in lines if line.startswith("compact-compile-jobs:")]}
    print(actor, "native", directory.parent.name, seconds, "reused", len(reused), flush=True)
    return result


def complete(actor, directory, profile=False):
    if actor in mech_actors:
        measure.runtime, measure.reference_pins = runtime_for(actor, profile)
    result = measure.measure("mech" if actor in mech_actors else actor, directory, profile=profile)
    result["actor"] = actor
    result["commands"] = json.loads((directory / "commands.json").read_text())
    if actor in mech_actors:
        result["effective_native_flags"] = measure.runtime["flags"]
        phases = json.loads((directory / "frontend/compile.json").read_text())["phases"]
        result["seconds"]["parse"] = next(row["seconds"] for row in phases if row["phase"] == "parse")
    return result


output.mkdir()
native_order = ("mech-smaller-control", "mech-smaller-reuse")
native_warmup = [native(actor, output / "native-warmup" / actor) for actor in native_order]
native_pairs = [list(native_order if i % 2 == 0 else native_order[::-1]) for i in range(8)]
native_trials = [[native(actor, output / f"native-order-{i}" / actor) for actor in pair]
                 for i, pair in enumerate(native_pairs, 1)]
native_profiles = [native(actor, output / "native-profile" / actor, True) for actor in mech_actors]
warmup = [complete(actor, output / "warmup" / actor) for actor in actors]
trials = [complete(actor, output / f"order-{i}" / actor) for i, order in enumerate(orders, 1) for actor in order]
profiles = [complete(actor, output / "profile" / actor, True) for actor in actors]

matches = ("frontend/program.mecb", "native/program.o", "native/metadata.json", "compressor")
for result in (*warmup, *trials, *profiles):
    actor = result["actor"]
    exe = Path(result["executable"])
    if actor in mech_actors:
        case, _ = mech_actors[actor]
        anchor = output / "warmup" / f"mech-{case}-control"
        for name in matches:
            assert (anchor / name).read_bytes() == (exe.parent / name).read_bytes(), (actor, name)
    else:
        measure.verify_peer(exe)
for actor in mech_actors:
    directory = output / "warmup" / actor
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", directory / "compressor",
                    directory / "native/metadata.json", f"equivalent-vector-{actor}"], check=True)

medians = {}
for actor in actors:
    samples = [row["seconds"] for row in trials if row["actor"] == actor]
    medians[actor] = {key: median(row[key] for row in samples) for key in samples[0]}
native_medians = {
    actor: median(row["seconds"] for pair in native_trials for row in pair if row["actor"] == actor)
    for actor in native_order
}
record = {
    "machine": "Apple M1", "measurement_boundary": "Encoder source to executable, including final link.",
    "source_sha256": recipe["source_sha256"], "bytecode_sha256": recipe["bytecode_sha256"],
    "frontend_sha256": measure.sha(measure.frontend), "emitter_sha256": measure.sha(measure.emitter),
    "fixed_runtime_sha256": measure.sha(Path(library)), "allocator_sha256": measure.sha(measure.mimalloc),
    "roc_compiler_sha256": measure.sha(measure.roc), "roc_source_sha256": measure.sha(measure.roc_app),
    "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
    "native_orders": native_pairs, "native_warmup": native_warmup, "native_trials": native_trials,
    "native_profiles": native_profiles, "native_medians": native_medians,
    "orders": orders, "warmup": warmup, "trials": trials, "profiles": profiles, "medians": medians,
    "identical_artifacts_within_each_configuration": matches, "verified_calls": 648,
}
(output / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps({"medians": medians, "native_medians": native_medians}, indent=2), flush=True)
