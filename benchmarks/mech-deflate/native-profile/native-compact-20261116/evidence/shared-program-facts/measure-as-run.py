"""Measure the shared immutable facts owner with all native outputs held exact."""
from argparse import ArgumentParser
from hashlib import file_digest
import json
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter

STUDY = Path(__file__).resolve().parent
Q = STUDY.parents[1]


def sha(path):
    with Path(path).open("rb") as f:
        return file_digest(f, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def put(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("output", type=Path)
    args = cli.parse_args()
    out = args.output.resolve()
    out.mkdir()
    base = Q / "refused-ssa-maintained/matched"
    prior = read(base / "plan.json")
    reference = read(base / "warmup/mech-experiment/result.json")
    source, link = (dict(reference["commands"])[name] for name in ("source_to_native", "link"))
    proof = read(STUDY / "rebuild/rebuild.json")
    old_object, = [arg for arg in link if arg.endswith("/native/program.o")]
    old_exe = link[link.index("-o") + 1]
    flags = prior["effective_native_flags"]["mech-experiment"]
    env = {name: value for name, value in os.environ.items() if not name.startswith("MECH_")}
    hashes = reference["sha256"]
    assert (proof["object_sha256"], proof["metadata_sha256"]) == (hashes["program.o"], hashes["metadata.json"])
    compilers = {"control": Path(source[0]), "shared": STUDY / "target/release/compile_profile"}
    identities = {actor: sha(path) for actor, path in compilers.items()}
    assert identities["control"] == prior["compiler_sha256"]["mech-experiment"]
    assert identities["control"] != identities["shared"]
    assert sha(source[1]) == prior["mech_source_sha256"]
    orders = [("control", "shared"), ("shared", "control")] * 4
    plan = {
        "scope": "Apple M1, matched compile from unchanged source through final benchmark link",
        "change": "Use one local lazy owner for numeric read-packing and branch analysis.",
        "ablation": "The original compiler builds a fresh facts owner per consumer.",
        "orders": orders, "compilers": identities, "source": source[1], "source_sha256": sha(source[1]),
        "program_and_worker_sha256": hashes, "flags": flags, "link": link, "patch_sha256": sha(STUDY / "source.patch"),
    }
    put(out / "plan.json", plan)

    def measure(actor, round_name):
        destination = out / round_name / actor
        destination.mkdir(parents=True)
        compiled = destination / "frontend"
        exe = destination / "compressor"
        compile_command = [str(compilers[actor]), source[1], str(compiled), *source[3:]]
        link_command = [str(compiled / "program.o") if value == old_object
                        else str(exe) if value == old_exe else value for value in link]
        times = {}
        start = perf_counter()
        for label, command, environment in (("source_to_native", compile_command, env | flags),
                                             ("link", link_command, env)):
            with (destination / f"{label}.log").open("wb") as log:
                phase = perf_counter()
                subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
                times[label] = perf_counter() - phase
        times["source_through_link"] = perf_counter() - start
        for name, expected in hashes.items():
            path = exe if name == "compressor" else compiled / name
            assert sha(path) == expected, (actor, name, sha(path))
        record = read(compiled / "compile.json")
        for row in record["phases"]:
            times[f'compiler_{row["phase"]}'] = row["seconds"]
        result = {"times_seconds": times, "compile": compile_command, "link": link_command,
                  "artifacts": hashes, "compiler_phases": record["phases"]}
        put(destination / "result.json", result)
        print(round_name, actor, times["source_through_link"], times["compiler_native"], flush=True)
        return result

    warmups = {actor: measure(actor, "warmup") for actor in compilers}
    trials = [{actor: measure(actor, f"pair-{number}") for actor in order}
              for number, order in enumerate(orders, 1)]
    assert {actor: sha(path) for actor, path in compilers.items()} == identities
    medians = {actor: {stage: median(record[actor]["times_seconds"][stage] for record in trials)
                       for stage in result["times_seconds"]} for actor, result in warmups.items()}
    ratios = {
        key: [row["shared"]["times_seconds"][key] / row["control"]["times_seconds"][key] for row in trials]
        for key in ("source_through_link", "compiler_native")
    }
    report = {
        "plan_sha256": sha(out / "plan.json"), "medians_seconds": medians,
        "paired_ratios": ratios, "paired_median_ratios": {name: median(value) for name, value in ratios.items()},
        "warmups": warmups, "trials": trials,
    }
    put(out / "result.json", report)
    print(json.dumps({key: report[key] for key in ("medians_seconds", "paired_median_ratios")}, indent=2))


if __name__ == "__main__":
    main()
