"""Compare machine-size projection with a preselected compact inlining plan."""
from collections import Counter
from importlib import import_module
import json
import os
from pathlib import Path
from statistics import median
import subprocess
import sys
from time import perf_counter

measure = import_module("compare-builds")
q = measure.q
study = q / "native-root-inline-plan"
packet = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
recipe = json.loads((packet / "recipe.json").read_text())
measure.source_mech = packet / "program.mec"
measure.frontend = q / "dead-spill-stores/native_compact_frontend"
measure.emitter = q / "host-opt-levels/opt3-cargo/release/emit_compact_size"
assert measure.sha(measure.source_mech) == recipe["source_sha256"]
assert measure.sha(measure.roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
assert measure.sha(measure.source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"

modes = {"mech-projection": "1", "mech-preselected": "preselected"}
reference = q / "stale-cache-accounting/pinned-smaller"
runtime = json.loads((reference / "runtime.json").read_text())
assert runtime["image_sha256"] == recipe["bytecode_sha256"]
assert runtime["flags"]["MECH_COMPACT_INLINE_SINGLE_CALLS"] == "1"
assert not any("LLVM" in k for k in runtime["flags"])
measure.reference_pins = json.loads((reference / "receipt.json").read_text())
library, = (Path(arg.removeprefix("mech_gpu=")) for arg in runtime["link"] if arg.startswith("mech_gpu="))
measure.mech_link = [library if str(value).endswith("/libmech_gpu_runtime.rlib") else value
                    for value in measure.mech_link]
fixed = {
    str(path): measure.sha(path)
    for path in (
        *(Path(arg) for arg in measure.mech_link if str(arg).endswith((".rlib", ".o"))
          and arg != str(measure.reference / "program.o")),
        *(measure.setup / name for name in ("roc-adapter.o", "libdeflate-adapter.o", "libdeflate-alloc.o", "libdeflate-arm.o")),
    )
}
tools = {name: {"path": str(path), "sha256": measure.sha(path)}
         for name, path in (("mech_frontend", measure.frontend), ("mech_emitter", measure.emitter),
                            ("roc", measure.roc), ("clang_driver", Path(measure.cc[0])))}
actors = (*modes, "roc", "libdeflate")
pattern = (0, 1, 3, 2)
orders = [[actors[(i + shift) % len(actors)] for i in pattern] for shift in range(len(actors))] * 2
assert all(Counter(position) == Counter({a:2 for a in actors}) for position in zip(*orders))
assert len(Counter((x, y) for order in orders for x,y in zip(order,order[1:]))) == 12
output = study / "matched"
ordinary_run = measure.run


def complete(actor, directory, profile=False):
    if actor in modes:
        measure.runtime = runtime | {"flags": runtime["flags"] |
            {"MECH_COMPACT_INLINE_SINGLE_CALLS": modes[actor]}}
    process_rss = {}

    def run(command, path, environment, instrument=False):
        start = perf_counter()
        markers = []
        with path.open("wb") as log:
            with subprocess.Popen(command, env=environment,
                                  stdout=subprocess.PIPE if instrument else log,
                                  stderr=subprocess.STDOUT) as worker:
                if instrument:
                    for line in worker.stdout:
                        if line.startswith(b"compact-stage:"):
                            markers.append([perf_counter()-start,line.decode().strip()])
                        log.write(line)
                _, stats, usage = os.wait4(worker.pid,0)
                worker.returncode = os.waitstatus_to_exitcode(stats)
                if worker.returncode:
                    raise subprocess.CalledProcessError(worker.returncode,command)
            elapsed = perf_counter()-start
        if instrument:
            path.with_suffix(".stages.json").write_text(json.dumps(markers,indent=2)+"\n")
        process_rss[path.stem] = usage.ru_maxrss
        return elapsed

    measure.run = run
    try:
        result = measure.measure("mech" if actor in modes else actor, directory, profile=profile)
    finally:
        measure.run = ordinary_run
    result["actor"] = actor
    result["commands"] = json.loads((directory/"commands.json").read_text())
    result["process_rss_bytes"] = process_rss
    result["executable_file_bytes"] = (directory/"compressor").stat().st_size
    if actor in modes:
        phases = json.loads((directory/"frontend/compile.json").read_text())["phases"]
        result["seconds"]["parse"] = next(row["seconds"] for row in phases if row["phase"]=="parse")
        result["effective_native_flags"] = measure.runtime["flags"]
        log = (directory/"native.log").read_text()
        markers = [line for line in log.splitlines() if line.startswith("compact native inline projection:")
                   or line.startswith("compact native preselected inline:")]
        assert len(markers)==1, (actor,markers)
        assert ("preselected inline:" in markers[0]) == (actor=="mech-preselected")
        result["root_inlining_decision"] = markers[0]
        result["root_machine_compilations"] = 1 if actor=="mech-preselected" else 2
        result["root_machine_compilations_scope"] = "Initial and optional inlined checked root, excluding preparatory IR work and helper compilation"
    return result


output.mkdir()
warmups = [complete(actor, output/"warmup"/actor) for actor in actors]
trials = [complete(actor, output/f"order-{number}"/actor)
          for number,order in enumerate(orders,1) for actor in order]
profiles = [complete(actor, output/"profiles"/actor, True) for actor in actors]
previous = q/"stale-cache-accounting/matched/warmup/mech-smaller-accounting/compressor"
anchored = warmups[0]["executable_sha256"]
assert measure.sha(previous) == anchored
for row in (*warmups,*trials,*profiles):
    exe = Path(row["executable"])
    if row["actor"] in modes:
        assert row["executable_sha256"] == anchored
    else:
        measure.verify_peer(exe)
for actor in modes:
    directory=output/"warmup"/actor
    subprocess.run([sys.executable,"-B",q/"verify-worker.py",directory/"compressor",
                    directory/"native/metadata.json",f"root-inline-plan-full-{actor}"],check=True)
timings = {
    actor: {
        phase: median([row["seconds"][phase] for row in trials if row["actor"]==actor])
        for phase in next(row["seconds"].keys() for row in trials if row["actor"]==actor)
    } for actor in actors
}
rss = {
    actor: {phase: median([row["process_rss_bytes"][phase] for row in trials if row["actor"]==actor])
            for phase in next(row["process_rss_bytes"].keys() for row in trials if row["actor"]==actor)}
    for actor in actors
}
pairs = [{row["actor"]:row for row in trials[i:i+4]} for i in range(0,len(trials),4)]
paired_ratios = [row["mech-preselected"]["seconds"]["total"]/row["mech-projection"]["seconds"]["total"] for row in pairs]
report = {"actors":actors,"orders":orders,"compiler_tools":tools,"fixed_inputs":fixed,
          "warmups":warmups,"trials":trials,"profiles":profiles,"complete_medians_seconds":timings,
          "median_process_rss_bytes":rss,"paired_total_ratios":paired_ratios}
(output/"timing.json").write_text(json.dumps(report,indent=2)+"\n")
print(json.dumps({"complete_medians_seconds":timings,"median_process_rss_bytes":rss,
                  "paired_total_ratios":paired_ratios},indent=2),flush=True)
