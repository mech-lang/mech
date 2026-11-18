"""Match pooled-frame compiler builds through the final executable link."""

from collections import Counter
from importlib import import_module
import json
from statistics import median
import subprocess
import sys

from study_tools import q, sha, study, write

sys.path.insert(0, str(q / "matched-source-builds"))
build = import_module("compare-direct-source")
measure = build.measure
output = study / "matched"
output.mkdir()
reference = q / "native-size-20261117/source-aligned"
runtime = json.loads((reference / "runtime.json").read_text())
assert runtime["image_sha256"] == build.recipe["bytecode_sha256"]
runtime["flags"]["MECH_COMPACT_INLINE_SINGLE_CALLS"] = "preselected"
assert not any("LLVM" in key for key in runtime["flags"])
runtime["expected"] = {
    "program.mecb": runtime["image_sha256"],
    **{name: sha(reference / name) for name in ("program.o", "metadata.json", "mech-worker")},
}
fixed_runtime, = [path for path in runtime["link"] if path.startswith("mech_gpu=")]
assert fixed_runtime.split("=", 1)[1] in build.mech_link_inputs
compilers = {
    "mech-control": q / "ordered-vector-cursors/before/compile_profile",
    "mech-experiment": q / "host-opt-levels/opt3-cargo/release/compile_profile",
}
identities = {name: sha(binary) for name, binary in compilers.items()}
assert len(set(identities.values())) == 2
build.variants = dict.fromkeys(compilers, runtime)
build.mech_actors = {name: (name, "direct") for name in compilers}
actors = (*compilers, "roc", "libdeflate")
pattern = (0, 1, 3, 2)
orders = [[actors[(i + shift) % len(actors)] for i in pattern] for shift in range(len(actors))] * 2
assert len(Counter((a, b) for order in orders for a, b in zip(order, order[1:]))) == 12
plan = {
    "scope": "Apple M1. Source through the final executable link with prebuilt compilers, fixed runtimes and adapters.",
    "compiler_sha256": identities, "mech_source_sha256": sha(measure.source_mech),
    "roc_compiler_sha256": sha(measure.roc), "roc_source_sha256": sha(measure.roc_app),
    "c_source_sha256": sha(measure.source_c / "lib/deflate_compress.c"),
    "clang": subprocess.check_output([measure.cc[0], "--version"], text=True).splitlines()[0],
    "effective_native_flags": runtime["flags"], "expected_artifacts": runtime["expected"],
    "fixed_link_input_sha256": build.mech_link_inputs | build.peer_link_inputs,
    "orders": orders,
}
write(output / "plan.json", plan)


def sample(actor, phase, profile=False):
    if actor in compilers:
        build.direct = compilers[actor]
    return build.complete(actor, output / phase / actor, profile)


warmups = {actor: sample(actor, "warmup") for actor in actors}
trials = [{actor: sample(actor, f"order-{i}") for actor in order}
          for i, order in enumerate(orders, 1)]
profiles = {actor: sample(actor, "profile", True) for actor in actors}
control_image = warmups["mech-control"]["sha256"]
for cohort in (warmups, *trials, profiles):
    for actor, item in cohort.items():
        if actor in compilers:
            assert item["sha256"] == control_image, (actor, item["sha256"])
        else:
            for key in ("encoder_sha256", "executable_sha256"):
                assert item[key] == warmups[actor][key], (actor, key)
            measure.verify_peer(item["executable"])

build.variants["parser-frame-pool"] = runtime
warm = output / "warmup/mech-experiment"
historical = build.historical_worker("parser-frame-pool", warm, warm / "native/program.o",
                                     warm / "native/metadata.json")
sys.path.insert(0, str(q))
macho = import_module("reachability-census")
sizes = {}
medians = {}
for actor in actors:
    work = output / "warmup" / actor
    exe = work / "compressor"
    if actor == "roc":
        library = work / "encoder.a"
        members = subprocess.check_output(["/usr/bin/ar", "-t", library], text=True).splitlines()
        image, = [name for name in members if name.startswith("roc_app_")]
        obj = work / "encoder-code.o"
        obj.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", library, image]))
    else:
        obj = work / ("native/program.o" if actor in compilers else "encoder.o")
    sizes[actor] = {
        "encoder_text_bytes": macho.load(obj)[1][macho.TEXT][2], "encoder_object_bytes": obj.stat().st_size,
        "executable_text_bytes": macho.load(exe)[1][macho.TEXT][2], "executable_bytes": exe.stat().st_size,
    }
    rows = [row[actor] for row in trials]
    result = {phase: median(row["seconds"][phase] for row in rows) for phase in rows[0]["seconds"]}
    result["compiler_peak_rss_bytes"] = median(max(
        value for name, value in row["phase_peak_rss_bytes"].items() if name != "link"
    ) for row in rows)
    if actor in compilers:
        phases = rows[0]["frontend_receipt"]["phases"]
        result["compiler_phases_seconds"] = {phase["phase"]: median(next(
            item["seconds"] for item in row["frontend_receipt"]["phases"] if item["phase"] == phase["phase"]
        ) for row in rows) for phase in phases}
    medians[actor] = result
assert [sizes[actor]["encoder_text_bytes"] for actor in actors] == [71768, 71768, 57696, 37080]
paired = [epoch["mech-experiment"]["seconds"]["total"] / epoch["mech-control"]["seconds"]["total"]
          for epoch in trials]
result = {
    "medians": medians, "sizes": sizes, "paired_complete_build_ratio": paired,
    "paired_complete_build_ratio_median": median(paired), "historical_worker": historical,
}
write(output / "timing.json", plan | {
    "warmups": warmups, "trials": trials, "profiles": profiles, **result,
})
print(json.dumps(result, indent=2))
