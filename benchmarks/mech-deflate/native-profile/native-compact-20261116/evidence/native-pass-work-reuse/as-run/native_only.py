"""Time native generation from fixed bytecode with both pinned encoder recipes."""

from collections import Counter
from hashlib import file_digest
import json
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter

study = Path(__file__).resolve().parent
q = study.parent
output = study / "native-only"
output.mkdir()
compiler = {
    "control": study / "before/emit_compact_size",
    "work-reuse": q / "host-opt-levels/opt3-cargo/release/emit_compact_size",
}
bytecode = q / "parser-queue-snapshot-20261117/identity/frontend-combined/program.mecb"


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


variants = {}
for name in ("retained", "smaller"):
    folder = q / f"stale-cache-accounting/pinned-{name}"
    runtime = json.loads((folder / "runtime.json").read_text())
    identity = json.loads((folder / "receipt.json").read_text())
    assert sha(bytecode) == runtime["image_sha256"]
    settings = runtime["flags"] | {"MECH_COMPACT_INLINE_SINGLE_CALLS":"preselected"}
    assert not any("LLVM" in option for option in settings)
    variants[name] = {"flags":settings,"object_sha256":identity["object_sha256"],
                      "metadata_sha256":identity["metadata_sha256"]}
common_env = {name:value for name,value in os.environ.items() if not name.startswith("MECH_")} | {"PATH":"/usr/bin:/bin"}
keys = {f"{recipe}-{role}":(recipe,role) for recipe in variants for role in compiler}
actors = tuple(keys)
orders = [[actors[(position + shift) % len(actors)] for position in (0, 1, 3, 2)]
          for shift in range(len(actors))] * 2
assert all(Counter(column) == Counter(dict.fromkeys(actors, 2)) for column in zip(*orders))
assert len({(a,b) for order in orders for a,b in zip(order,order[1:])}) == 12
plan = {"scope":"Fixed bytecode through native object and complete metadata", "bytecode_sha256":sha(bytecode),
        "compiler_sha256":{role:sha(path) for role,path in compiler.items()}, "variants":variants, "orders":orders}
(output / "plan.json").write_text(json.dumps(plan,indent=2) + "\n")


def trial(actor, label):
    recipe, role = keys[actor]
    variant = variants[recipe]
    folder = output / label / actor
    folder.mkdir(parents=True,exist_ok=False)
    argv = [compiler[role], bytecode, folder]
    with (folder / "native.log").open("wb") as log:
        start = perf_counter()
        with subprocess.Popen(argv, env=common_env | variant["flags"], stdout=log, stderr=subprocess.STDOUT) as child:
            _, status, usage = os.wait4(child.pid,0)
            child.returncode = os.waitstatus_to_exitcode(status)
            if child.returncode:
                raise subprocess.CalledProcessError(child.returncode,argv)
        elapsed = perf_counter() - start
    object_sha = sha(folder / "program.o")
    metadata_sha = sha(folder / "metadata.json")
    assert object_sha == variant["object_sha256"] and metadata_sha == variant["metadata_sha256"], actor
    print(actor,label,elapsed,usage.ru_maxrss,flush=True)
    return {"actor":actor,"command":list(map(str,argv)), "seconds":elapsed,"peak_rss_bytes":usage.ru_maxrss,
            "object_sha256":object_sha,"metadata_sha256":metadata_sha}


warmups = {actor:trial(actor,"warmup") for actor in actors}
trials = [{actor:trial(actor,f"order-{n}") for actor in order} for n,order in enumerate(orders,1)]
results = {"orders":orders,"warmups":warmups,"trials":trials}
results["median_seconds"] = {actor:median(row[actor]["seconds"] for row in trials) for actor in actors}
results["median_peak_rss_bytes"] = {actor:median(row[actor]["peak_rss_bytes"] for row in trials) for actor in actors}
(output / "timing.json").write_text(json.dumps(results,indent=2)+"\n")
print(json.dumps(results["median_seconds"],indent=2))
