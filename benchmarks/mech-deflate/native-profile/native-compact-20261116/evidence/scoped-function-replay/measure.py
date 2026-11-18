"""Compare function replay through the same deployment link."""
from argparse import ArgumentParser
from importlib import import_module
from pathlib import Path
from statistics import median
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
driver = import_module("measure-deployment-builds")
PREVIOUS = HERE.parent / "definition-compiler"
ACTORS = ("mech-control", "mech-function-replay", "roc", "libdeflate")


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("output", type=Path)
    args = cli.parse_args()
    output = args.output.resolve()
    output.mkdir()
    base = driver.MATCHED
    original = driver.read(base / "plan.json")
    warm = base / "warmup"
    template = dict(driver.read(warm / "mech-experiment/result.json")["commands"])["source_to_native"]
    assert driver.sha(template[1]) == original["mech_source_sha256"]
    flags = original["effective_native_flags"]["mech-experiment"]
    expected = original["expected_artifacts"]["mech-experiment"]
    deploy = driver.read(driver.STOCK / "build.json")
    slim = driver.read(driver.SMALL / "receipt.json")["commands"]["link"]
    object_pin, = [value for value in slim if value.endswith("/program.o")]
    pins = {"encoder": expected["program.o"], "bytecode": expected["program.mecb"], "metadata": expected["metadata.json"]}
    bindings = {}
    for actor, work in zip(ACTORS[:2], (PREVIOUS / "candidate", HERE / "candidate"), strict=True):
        exe = work / "compile_profile"
        assert driver.sha(exe) == (work / "compiler.sha256").read_text().strip()
        result = driver.read(work / "rebuild/rebuild.json")
        assert (result["object_sha256"], result["metadata_sha256"]) == (pins["encoder"], pins["metadata"])
        bindings[actor] = [str(exe), *template[1:]], slim, object_pin, flags, pins
    assert driver.sha(bindings[ACTORS[0]][0][0]) != driver.sha(bindings[ACTORS[1]][0][0])
    for actor, ext in (("roc", "a"), ("libdeflate", "o")):
        source = dict(driver.read(warm / actor / "commands.json"))["compile"]
        obj = str(warm / actor / f"encoder.{ext}")
        bindings[actor] = source, deploy["commands"][f"link-{actor}"], obj, {}, {"encoder": driver.sha(obj)}
    roc_source = bindings["roc"][0][-1]
    c_source, = [arg for arg in bindings["libdeflate"][0] if arg.endswith("/deflate_compress.c")]
    assert driver.sha(roc_source) == original["roc_source_sha256"]
    assert driver.sha(c_source) == original["c_source_sha256"]
    assert driver.sha(bindings["roc"][0][0]) == original["roc_compiler_sha256"]
    paths = (0, 1, 3, 2)
    orders = [[ACTORS[(index + shift) % 4] for index in paths] for shift in range(4)] * 2
    variants = {}
    for actor, (src, link, obj, env, digests) in bindings.items():
        link = [arg for arg in link if not arg.startswith("-Wl,-map,")]
        bindings[actor] = src, link, obj, env, digests
        variants[actor] = {
            "compiler_sha256": driver.sha(src[0]), "source_command": src, "link_command": link,
            "native_flags": env, "expected_artifacts": digests,
            "fixed_link_inputs": {arg: driver.sha(arg) for arg in link[:link.index("-o")]
                                  if arg != obj and arg.endswith((".o", ".a", ".rlib"))},
        }
    plan = {
        "scope": "Same Apple M1 source-through-deployment compiler, provider archives prebuilt",
        "change": "Use the guarded definition recipe for selected function definitions, with settled nested raw witnesses",
        "source_patch_sha256": driver.sha(HERE / "candidate/function-replay.patch"),
        "source_sha256": {"mech": original["mech_source_sha256"],
                          "roc": original["roc_source_sha256"], "libdeflate": original["c_source_sha256"]},
        "deployment_driver_sha256": driver.sha(Path(driver.__file__)),
        "order": orders, "warmup": ACTORS, "variants": variants,
        "compiler_source_parent": "ff7c1dbc9c5059ed84d730b008847c02179fa2cf",
        "same_mech_deployment_link": True, "same_exact_mech_encoder_and_metadata": True,
    }
    driver.save(output / "plan.json", plan)

    def sample(actor, phase):
        return driver.compile_and_link(actor, output / phase / actor, *bindings[actor])

    warmups = {actor: sample(actor, "warmup") for actor in ACTORS}
    trials = [{actor: sample(actor, f"order-{i}") for actor in order}
              for i, order in enumerate(orders, 1)]
    candidate = ACTORS[1]
    for trial in trials:
        assert trial[candidate]["stripped_executable"] == trial["mech-control"]["stripped_executable"]
    medians = {
        actor: {stage: median(trial[actor]["seconds"][stage] for trial in trials)
                for stage in warmups[actor]["seconds"]}
        for actor in ACTORS
    }
    stages = {
        actor: {row["phase"]: median(next(item["seconds"] for item in trial[actor]["compiler_phases"]
                                          if item["phase"] == row["phase"]) for trial in trials)
                for row in warmups[actor]["compiler_phases"]} for actor in ACTORS[:2]
    }
    ratio = {peer: median(trial[candidate]["seconds"]["source_through_link"] /
                          trial[peer]["seconds"]["source_through_link"] for trial in trials)
             for peer in ("mech-control", "roc", "libdeflate")}
    data = {
        "plan_sha256": driver.sha(output / "plan.json"), "medians_seconds": medians,
        "compiler_phase_medians_seconds": stages, "candidate_paired_build_x": ratio,
        "compiler_peak_rss_bytes": {
            actor: median(trial[actor]["phase_peak_rss_bytes"]["source"] for trial in trials)
            for actor in ACTORS
        },
        "warmups": warmups, "trials": trials,
    }
    driver.save(output / "comparison.json", data)
    print(data["medians_seconds"], flush=True)
    print(data["candidate_paired_build_x"], flush=True)


if __name__ == "__main__":
    main()
