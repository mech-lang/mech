"""Time three unchanged encoder sources through the matched deployment links."""
from argparse import ArgumentParser
from collections import Counter
from hashlib import file_digest
import json
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter


Q = Path(__file__).resolve().parents[1]
MATCHED = Q / "refused-ssa-maintained/matched"
STOCK = Q / "minimal-deployment/verified"
SMALL = Path("/private/tmp/mech-root-transport.A9Qtzt/mech-alloc-only/trial")
ACTORS = ("mech-std", "mech-core-alloc", "roc", "libdeflate")


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with Path(path).open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def replace_arg(argv, old, new):
    assert argv.count(str(old)) == 1, (old, argv)
    return [str(new) if arg == str(old) else arg for arg in argv]


def compile_and_link(actor, trial, source_argv, link_argv, pinned_encoder, extra_flags, pinned_bytes):
    trial.mkdir(parents=True)
    mech = actor.startswith("mech-")
    frontend = trial / "frontend"
    encoder = frontend / "program.o" if mech else trial / Path(pinned_encoder).name
    exe = trial / "compressor"
    if mech:
        compile_command = replace_arg(source_argv, source_argv[2], frontend)
    elif actor == "roc":
        compile_command = replace_arg(source_argv, f"--output={pinned_encoder}", f"--output={encoder}")
    else:
        compile_command = replace_arg(source_argv, pinned_encoder, encoder)
    link_command = replace_arg(link_argv, pinned_encoder, encoder)
    link_command = replace_arg(link_command, link_command[link_command.index("-o") + 1], exe)
    common = {name: value for name, value in os.environ.items() if not name.startswith("MECH_")}
    source_env = common | extra_flags
    if actor == "roc":
        source_env["ROC_CACHE_DIR"] = str(trial / "roc-cache")
    commands = {"source": compile_command, "link": link_command,
                "strip": ["/usr/bin/strip", "-S", "-x", str(exe)]}
    times = {}

    def run(phase, env):
        with (trial / f"{phase}.log").open("wb") as log:
            start = perf_counter()
            subprocess.run(commands[phase], env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            times[phase] = perf_counter() - start

    start = perf_counter()
    run("source", source_env)
    run("link", common)
    times["source_through_link"] = perf_counter() - start
    run("strip", common)
    times["source_through_strip"] = perf_counter() - start
    assert sha(encoder) == pinned_bytes["encoder"], (actor, sha(encoder))
    if mech:
        assert sha(frontend / "program.mecb") == pinned_bytes["bytecode"]
        assert sha(frontend / "metadata.json") == pinned_bytes["metadata"]
        phase_receipt = read(frontend / "compile.json")["phases"]
    else:
        phase_receipt = None
    result = {
        "seconds": times, "commands": commands,
        "encoder_sha256": pinned_bytes["encoder"], "generated_artifacts": pinned_bytes,
        "stripped_executable": {"bytes": exe.stat().st_size, "sha256": sha(exe)},
        "compiler_phases": phase_receipt,
    }
    save(trial / "result.json", result)
    print(actor, trial.parent.name, round(times["source_through_link"], 5),
          round(times["source_through_strip"], 5), exe.stat().st_size, flush=True)
    return result


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("output", type=Path)
    args = cli.parse_args()
    output = args.output.resolve()
    output.mkdir()
    current = read(MATCHED / "plan.json")
    warm = MATCHED / "warmup"
    original = read(warm / "mech-experiment/result.json")
    stock = read(STOCK / "build.json")
    slim = read(SMALL / "receipt.json")
    mech_source = dict(original["commands"])["source_to_native"]
    assert sha(mech_source[0]) == current["compiler_sha256"]["mech-experiment"]
    assert sha(mech_source[1]) == current["mech_source_sha256"]
    fixed_flags = current["effective_native_flags"]["mech-experiment"]
    expected = current["expected_artifacts"]["mech-experiment"]
    object_pin = next(arg for arg in stock["commands"]["link-mech"] if arg.endswith("/program.o"))
    bindings = {
        "mech-std": (mech_source, stock["commands"]["link-mech"], object_pin, fixed_flags),
        "mech-core-alloc": (mech_source, slim["commands"]["link"], object_pin, fixed_flags),
    }
    digests = dict.fromkeys(bindings, {"encoder": expected["program.o"], "bytecode": expected["program.mecb"],
                                     "metadata": expected["metadata.json"]})
    for actor, extension in (("roc", "a"), ("libdeflate", "o")):
        source_command = dict(read(warm / actor / "commands.json"))["compile"]
        old_object = str(warm / actor / f"encoder.{extension}")
        assert old_object in stock["commands"][f"link-{actor}"]
        digests[actor] = {"encoder": sha(old_object)}
        bindings[actor] = (source_command, stock["commands"][f"link-{actor}"], old_object, {})
    assert sha(bindings["roc"][0][0]) == current["roc_compiler_sha256"]
    assert sha(bindings["roc"][0][-1]) == current["roc_source_sha256"]
    c_source, = [arg for arg in bindings["libdeflate"][0] if arg.endswith("/deflate_compress.c")]
    assert sha(c_source) == current["c_source_sha256"]
    pattern = (0, 1, 3, 2)
    orders = [[ACTORS[(i + shift) % len(ACTORS)] for i in pattern] for shift in range(len(ACTORS))] * 2
    assert len(Counter((a, b) for order in orders for a, b in zip(order, order[1:]))) == 12
    emit = {}
    for actor, (cmd, link, obj, flags) in bindings.items():
        link = [arg for arg in link if not arg.startswith("-Wl,-map,")]
        bindings[actor] = cmd, link, obj, flags
        emit[actor] = {
            "compiler": {"path": cmd[0], "sha256": sha(cmd[0])},
            "source_invocation": cmd, "link_invocation": link, "encoder": digests[actor],
            "native_flags": flags, "fixed_link_inputs": {
                arg: sha(arg) for arg in link[:link.index("-o")]
                if arg != obj and arg.endswith((".a", ".o", ".rlib"))
            },
        }
    plan = {
        "scope": "Matched Apple arm64 release encoders built from source through common deployment link "
                 "and common symbol strip. Fixed host, compiler and allocator inputs prebuilt.",
        "comparison_inputs": emit, "warmup_order": ACTORS, "orders": orders,
        "timing": "Wall time. Fresh output/cache per actor, serial balanced order. Link-map and oracle checks are untimed.",
        "retained_compiler_plan_sha256": sha(MATCHED / "plan.json"),
    }
    save(output / "plan.json", plan)

    def sample(actor, phase):
        return compile_and_link(actor, output / phase / actor, *bindings[actor], digests[actor])

    warmups = {actor: sample(actor, "warmup") for actor in ACTORS}
    trials = [{actor: sample(actor, f"order-{n}") for actor in order}
              for n, order in enumerate(orders, 1)]
    assert all(item["stripped_executable"]["bytes"] == warmups[actor]["stripped_executable"]["bytes"]
               for trial in trials for actor, item in trial.items())
    medians = {
        actor: {stage: median(trial[actor]["seconds"][stage] for trial in trials)
                for stage in warmups[actor]["seconds"]}
        for actor in ACTORS
    }
    ratios = {peer: median(trial["mech-core-alloc"]["seconds"]["source_through_link"] /
                           trial[peer]["seconds"]["source_through_link"] for trial in trials)
              for peer in ("mech-std", "roc", "libdeflate")}
    data = {"plan_sha256": sha(output / "plan.json"), "medians_seconds": medians,
            "paired_mech_core_alloc_build_x_peer": ratios, "warmups": warmups, "trials": trials}
    save(output / "timing.json", data)
    print(json.dumps({"medians": medians, "paired_ratios": ratios}, indent=2), flush=True)


if __name__ == "__main__":
    main()
