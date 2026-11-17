"""Compare native emitters and complete encoder builds on the retained workload."""

from importlib import import_module
import json
from pathlib import Path
from statistics import median
import subprocess
import sys
from time import perf_counter

measure = import_module("compare-builds")
q = measure.q
stage = q / "native-compile-stages-study"
out = measure.work / "apple-m1-batched-block-params"
out.mkdir()
retained = q / "cranelift-continuation-registers-word-pages-native/final-candidate"
measure.runtime = json.loads((retained / "runtime.json").read_text())
measure.reference_pins = json.loads((retained / "receipt.json").read_text())
binaries = q / "native-review-pr/baseline-build/compiler/release"
measure.frontend = binaries / "native_compact_frontend"
emitters = {
    "mech-control": stage / "control-emitter",
    "mech-batch": binaries / "emit_compact_size",
}
library, = (v.removeprefix("mech_gpu=") for v in measure.runtime["link"] if v.startswith("mech_gpu="))
measure.mech_link = [library if p.endswith("/libmech_gpu_runtime.rlib") else p for p in measure.mech_link]
image = measure.source_mech.parent / "program.mecb"
assert measure.sha(image) == measure.runtime["image_sha256"]
environment = measure.env | measure.runtime["flags"]
assert not any("LLVM" in key for key in measure.runtime["flags"])


def native(actor, directory, diagnostic=False):
    directory.mkdir(parents=True)
    cmd = [emitters[actor], image, directory]
    events = []
    log_path = directory / "emit.log"
    if diagnostic:
        with log_path.open("wb") as log:
            start = perf_counter()
            with subprocess.Popen(cmd, env=environment, stdout=subprocess.PIPE,
                                  stderr=subprocess.STDOUT) as child:
                for line in child.stdout:
                    if line.startswith((b"compact-stage:", b"compact vector 362 array",
                                        b"compact machine inline projection")):
                        events.append([perf_counter() - start, line.decode().strip()])
                    log.write(line)
                if child.wait():
                    raise subprocess.CalledProcessError(child.returncode, cmd)
            elapsed = perf_counter() - start
    else:
        elapsed = measure.run(cmd, log_path, environment)
    identity = {path: measure.sha(directory / path) for path in ["program.o", "metadata.json"]}
    assert identity["program.o"] == measure.reference_pins["object_sha256"]
    assert identity["metadata.json"] == measure.reference_pins["metadata_sha256"]
    record = {
        "actor": actor, "seconds": elapsed, "cwd": str(Path.cwd()), "command": list(map(str, cmd)),
        "output": identity, "diagnostic": diagnostic, "events": events,
    }
    (directory / "result.json").write_text(json.dumps(record, indent=2) + "\n")
    print("native", actor, directory.parent.name, elapsed, flush=True)
    return record


def complete(actor, directory, diagnostic=False):
    if actor in emitters:
        measure.emitter = emitters[actor]
    record = measure.measure("mech" if actor in emitters else actor, directory, profile=diagnostic)
    record["actor"] = actor
    if actor in emitters:
        parsed = json.loads((directory / "frontend/compile.json").read_text())
        record["seconds"]["parse"] = next(p["seconds"] for p in parsed["phases"] if p["phase"] == "parse")
    (directory / "result.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


native_warmup = [native(actor, out / "native-warmup" / actor) for actor in emitters]
native_trials = []
for order in range(1, 9):
    cohort = tuple(emitters) if order % 2 else tuple(reversed(emitters))
    native_trials.append([native(actor, out / f"native-{order}" / actor) for actor in cohort])
native_profiles = []
for order, cohort in enumerate((tuple(emitters), tuple(reversed(emitters))), 1):
    native_profiles.extend(native(actor, out / f"native-profile-{order}" / actor, True) for actor in cohort)

actors = [*emitters, "roc", "libdeflate"]
orders = ((0, 1, 3, 2), (1, 2, 0, 3), (2, 3, 1, 0), (3, 0, 2, 1))
warmup = [complete(actor, out / "warmup" / actor) for actor in actors]
trials = [complete(actors[i], out / f"order-{order}" / actors[i])
          for order, cohort in enumerate(orders, 1) for i in cohort]
profiles = [complete(actor, out / "profile" / actor, True) for actor in actors]

reference = out / "warmup/mech-control"
compared = ["frontend/program.mecb", "native/program.o", "native/metadata.json", "compressor"]
for row in [*warmup, *trials, *profiles]:
    executable = Path(row["executable"])
    if row["actor"] in emitters:
        for name in compared:
            assert (reference / name).read_bytes() == (executable.parent / name).read_bytes(), (row["actor"], name)
    else:
        measure.verify_peer(executable)
for actor in emitters:
    directory = out / "warmup" / actor
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", directory / "compressor",
                    directory / "native/metadata.json", f"matched-batched-params-{actor}"], check=True)

sys.path.insert(0, str(q))
macho = import_module("reachability-census")
sizes = {}
for actor in actors:
    directory = out / "warmup" / actor
    executable = directory / "compressor"
    if actor == "roc":
        archive = directory / "encoder.a"
        members = subprocess.check_output(["/usr/bin/ar", "-t", archive], text=True).splitlines()
        member, = [m for m in members if m.startswith("roc_app_")]
        encoder = directory / "encoder-code.o"
        encoder.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", archive, member]))
    else:
        encoder = directory / ("native/program.o" if actor in emitters else "encoder.o")
    _, sections, _ = macho.load(encoder)
    _, linked, _ = macho.load(executable)
    sizes[actor] = {
        "emitted_text_bytes": sections[macho.TEXT][2], "encoder_file_bytes": encoder.stat().st_size,
        "linked_text_bytes": linked[macho.TEXT][2], "executable_file_bytes": executable.stat().st_size,
        "encoder_sha256": measure.sha(encoder),
    }

medians = {}
for actor in actors:
    rows = [row["seconds"] for row in trials if row["actor"] == actor]
    assert len(rows) == 4
    medians[actor] = {key: median(row[key] for row in rows) for key in rows[0]}
native_medians = {
    actor: median(row["seconds"] for pair in native_trials for row in pair if row["actor"] == actor)
    for actor in emitters
}
record = {
    "machine": "Apple M1",
    "scope": "Fresh encoder source through final executable link; compilers, runtimes and adapters prebuilt.",
    "mech_compiler_build": "Rust release opt-level 1, no PGO; shared frontend and fixed runtime.",
    "source_sha256": measure.sha(measure.source_mech),
    "frontend_sha256": measure.sha(measure.frontend),
    "emitters": {k: {"path": str(v), "sha256": measure.sha(v)} for k, v in emitters.items()},
    "runtime_sha256": measure.sha(Path(library)),
    "roc_compiler_sha256": measure.sha(measure.roc), "roc_source_sha256": measure.sha(measure.roc_app),
    "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
    "native_warmup": native_warmup, "native_trials": native_trials, "native_medians": native_medians,
    "native_diagnostic_profiles": native_profiles,
    "complete_orders": [[actors[i] for i in order] for order in orders],
    "complete_warmup": warmup, "complete_trials": trials, "complete_medians": medians,
    "complete_diagnostic_profiles": profiles, "sizes": sizes,
    "byte_identical_across_both_emitters_and_all_complete_builds": compared,
    "verified_exact_calls": 324,
}
(out / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps({"native": native_medians, "complete": medians, "sizes": sizes}, indent=2), flush=True)
