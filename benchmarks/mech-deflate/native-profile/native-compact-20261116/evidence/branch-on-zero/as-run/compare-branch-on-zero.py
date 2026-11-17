"""Match direct source-through-link builds for the two zero-branch settings."""

from collections import Counter
from importlib import import_module
import json
from pathlib import Path
from statistics import median
import subprocess
import sys

build = import_module("compare-direct-source")
measure = build.measure
q = build.q
study = q / "native-size-20261117"
output = study / "matched"
output.mkdir()

folders = {
    "mech-control": q / "stale-cache-accounting/pinned-smaller",
    "mech-experiment": study / "source-aligned",
}
build.variants = {}
for name, folder in folders.items():
    runtime = json.loads((folder / "runtime.json").read_text())
    assert runtime["image_sha256"] == build.recipe["bytecode_sha256"]
    runtime["flags"]["MECH_COMPACT_INLINE_SINGLE_CALLS"] = "preselected"
    assert not any("LLVM" in key for key in runtime["flags"])
    build.variants[name] = runtime | {
        "expected": {
            "program.mecb": runtime["image_sha256"],
            "program.o": measure.sha(folder / "program.o"),
            "metadata.json": measure.sha(folder / "metadata.json"),
            "mech-worker": measure.sha(folder / "mech-worker"),
        },
    }
build.mech_actors = {name: (name, "direct") for name in folders}
actors = (*build.mech_actors, "roc", "libdeflate")
pattern = (0, 1, 3, 2)
orders = [[actors[(shift + index) % len(actors)] for index in pattern] for shift in range(len(actors))] * 2
assert len(Counter((first, second) for order in orders for first, second in zip(order, order[1:]))) == 12
plan = {
    "scope": "Apple M1 source through final executable link. Compilers, runtimes and adapters prebuilt.",
    "compiler_sha256": measure.sha(build.direct),
    "mech_source_sha256": measure.sha(measure.source_mech),
    "roc_compiler_sha256": measure.sha(measure.roc),
    "roc_source_sha256": measure.sha(measure.roc_app),
    "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
    "clang": subprocess.check_output([measure.cc[0], "--version"], text=True).splitlines()[0],
    "effective_native_flags": {name: runtime["flags"] for name, runtime in build.variants.items()},
    "reference_artifacts": {name: runtime["expected"] for name, runtime in build.variants.items()},
    "fixed_link_input_sha256": build.mech_link_inputs | build.peer_link_inputs,
    "orders": orders,
}
(output / "plan.json").write_text(json.dumps(plan, indent=2) + "\n")
warmups = [build.complete(actor, output / "warmup" / actor) for actor in actors]
rows = [build.complete(actor, output / f"order-{index}" / actor)
        for index, order in enumerate(orders, 1) for actor in order]
profiles = [build.complete(actor, output / "profile" / actor, True) for actor in actors]
for row in (*rows, *profiles):
    actor = row["actor"]
    anchor = next(item for item in warmups if item["actor"] == actor)
    if actor in folders:
        assert row["sha256"] == anchor["sha256"], actor
    else:
        for key in ("encoder_sha256", "executable_sha256"):
            assert row[key] == anchor[key], (actor, key)
        measure.verify_peer(Path(row["executable"]))

sys.path.insert(0, str(q))
macho = import_module("reachability-census")
medians = {}
sizes = {}
for actor in actors:
    samples = [row for row in rows if row["actor"] == actor]
    result = {key: median(sample["seconds"][key] for sample in samples) for key in samples[0]["seconds"]}
    result["compiler_peak_rss_bytes"] = median(
        max(value for key, value in sample["phase_peak_rss_bytes"].items() if key != "link")
        for sample in samples
    )
    if actor in folders:
        result["compiler_phases_seconds"] = {
            entry["phase"]: median(next(row["seconds"] for row in sample["frontend_receipt"]["phases"]
                                        if row["phase"] == entry["phase"]) for sample in samples)
            for entry in samples[0]["frontend_receipt"]["phases"]
        }
    medians[actor] = result
    destination = output / "warmup" / actor
    exe = destination / "compressor"
    if actor == "roc":
        archive = destination / "encoder.a"
        members = subprocess.check_output(["/usr/bin/ar", "-t", archive], text=True).splitlines()
        member, = (name for name in members if name.startswith("roc_app_"))
        obj = destination / "encoder-code.o"
        obj.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", archive, member]))
    else:
        obj = destination / ("native/program.o" if actor in folders else "encoder.o")
    _, object_sections, _ = macho.load(obj)
    _, executable_sections, _ = macho.load(exe)
    sizes[actor] = {"encoder_text_bytes": object_sections[macho.TEXT][2], "encoder_object_bytes": obj.stat().st_size,
                    "executable_text_bytes": executable_sections[macho.TEXT][2], "executable_bytes": exe.stat().st_size}

(output / "timing.json").write_text(json.dumps(plan | {
    "warmups": warmups, "trials": rows, "profiles": profiles, "medians": medians, "sizes": sizes,
}, indent=2) + "\n")
print(json.dumps(medians, indent=2), flush=True)
