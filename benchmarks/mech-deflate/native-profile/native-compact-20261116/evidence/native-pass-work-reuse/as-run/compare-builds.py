"""Time serial encoder source-to-executable builds with fixed host adapters."""
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
from time import perf_counter
import zlib

work = Path(__file__).resolve().parent
q = work.parent
setup = work / "setup"
source_root = q / "shared-huffman-packed-writer-shell-sort-keys-reachable-unmapped-native/frontend"
source_mech = source_root / "source.mec"
reference = q / "cranelift-literal-arguments-root-fold-native/final-candidate"
roc_app = work / "published-peer-inputs/peer-inputs/roc-profile/app.roc"
roc = next((work / "roc-compiler").iterdir()) / "zig-out/bin/roc"
source_c = next((work / "libdeflate").iterdir())
frontend = q / "compiler-rebuild-preflight/release/compile_profile"
emitter = q / "emit_compact_with_final_root_cleanup"
llvm_emitter = q / "emit_compact_with_llvm_lane_extract"
llvm_bin = Path("/opt/homebrew/opt/llvm/bin")
llvm_recipe = work / "pr973-compact/benchmarks/mech-deflate/native-profile/compact-size/rebuild.py"
llvm_sha = "a917aa339eb7291e0cbcb5fadedd2a4bf263df0db0ffe408f96310fa084b1f14"
reference_pins = json.loads((reference / "receipt.json").read_text())
runtime = json.loads((reference / "runtime.json").read_text())
mech_link = json.loads((setup / "mech-link-argv.json").read_text())
mimalloc = next(Path(x) for x in mech_link if x.endswith("/mimalloc.o"))
cc = ["/usr/bin/clang", "-arch", "arm64"]
env = {k: v for k, v in os.environ.items() if not k.startswith("MECH_")}
env["PATH"] = "/usr/bin:/bin"

def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def run(command, output, environment, instrument=False):
    with output.open("wb") as log:
        start = perf_counter()
        if instrument:
            events = []
            with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                  env=environment) as worker:
                for line in worker.stdout:
                    if line.startswith(b"compact-stage:"):
                        events.append([perf_counter() - start, line.decode().strip()])
                    log.write(line)
                if worker.wait():
                    raise subprocess.CalledProcessError(worker.returncode, command)
            elapsed = perf_counter() - start
            output.with_suffix(".stages.json").write_text(json.dumps(events,indent=2)+"\n")
        else:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=environment,
                           check=True)
            elapsed = perf_counter() - start
        return elapsed

def verify_peer(executable):
    fixtures = json.loads((q / "fixtures.json").read_text())
    for name in ("empty", "random-1024", "prefix-natural-8192"):
        case = next((v for v in fixtures if v["name"] == name), None)
        if case is None:
            continue
        input_path = Path(case["input"])
        result = subprocess.check_output([executable, input_path])
        prefix = Path(case["prefix"]).read_bytes()
        expected = Path(case["expected"]).read_bytes()[len(prefix):]
        assert result == expected, (executable, name)
        assert zlib.decompress(result, -15) == input_path.read_bytes(), (executable, name)

def llvm_profilers(destination):
    directory = destination / "llvm-tools"
    directory.mkdir()
    command = f"#!{sys.executable}\n" + r'''
import json
import os
from pathlib import Path
import subprocess
import sys
from time import perf_counter

tool = Path(sys.argv[0]).name
start = perf_counter()
status = subprocess.call([str(Path("/opt/homebrew/opt/llvm/bin") / tool), *sys.argv[1:]])
with open(os.environ["MATCHED_LLVM_PHASE_LOG"], "a") as stream:
    stream.write(json.dumps({"tool": tool, "seconds": perf_counter()-start,
                             "args": sys.argv[1:], "status": status}) + "\n")
raise SystemExit(status)
'''
    for name in ("llvm-link", "opt", "llc", "llvm-mc"):
        tool = directory / name
        tool.write_text(command)
        tool.chmod(0o755)
    return directory

def measure(actor, destination, profile=False):
    destination.mkdir(parents=True)
    native = destination / "native"
    if actor != "mech-llvm":
        native.mkdir()
    frontend_dir = destination / "frontend"
    executable = destination / "compressor"
    command_records = []
    timings = {}
    if actor.startswith("mech"):
        encoder = native / "program.o"
        commands = [("frontend", [frontend, source_mech, frontend_dir, "frontend"], env)]
        if actor == "mech":
            native_environment = env | runtime["flags"]
            commands.append(("native", [emitter, frontend_dir / "program.mecb", native], native_environment))
        else:
            llvm = llvm_profilers(destination) if profile else llvm_bin
            native_environment = env | ({"MATCHED_LLVM_PHASE_LOG": str(destination / "llvm-phases.jsonl")}
                                        if profile else {})
            commands.append(("native", [sys.executable, "-B", llvm_recipe, native, "--llvm-bin", llvm,
                                        "--program", frontend_dir / "program.mecb", "--emitter", llvm_emitter],
                             native_environment))
        link = [str(encoder) if value == str(reference / "program.o")
                else str(executable) if value == str(setup / "mech-adapter-check") else value
                for value in mech_link]
        assert str(encoder) in link and str(executable) in link
    elif actor == "roc":
        encoder = destination / "encoder.a"
        command = [roc, "build", "--no-cache", "--opt=speed", "--jobs=1", "--target=arm64mac",
                   f"--output={encoder}", roc_app]
        if profile:
            command.insert(2, "--timings")
        commands = [("compile", command, env | {"ROC_CACHE_DIR": str(destination / "roc-cache")})]
        link = [*cc, setup / "roc-adapter.o", encoder, mimalloc, "-o", executable]
    else:
        encoder = destination / "encoder.o"
        command = [*cc, "-std=gnu11", "-O2", "-DNDEBUG", "-Wall", "-Werror",
                   "-I", source_c, "-c", source_c / "lib/deflate_compress.c", "-o", encoder]
        if profile:
            command.extend((f"-ftime-trace={destination / 'clang-trace.json'}",
                            "-ftime-trace-granularity=25", "-ftime-report"))
        commands = [("compile", command, env)]
        link = [*cc, setup / "libdeflate-adapter.o", encoder, setup / "libdeflate-alloc.o",
                setup / "libdeflate-arm.o", mimalloc, "-o", executable]
    commands.append(("link", link, env))
    for phase, command, _ in commands:
        command_records.append([phase, list(map(str, command))])
    (destination / "commands.json").write_text(json.dumps(command_records, indent=2) + "\n")
    start = perf_counter()
    for phase, command, environment in commands:
        timings[phase] = run(command, destination / f"{phase}.log", environment,
                             instrument=profile and actor.startswith("mech") and phase == "native")
    timings["total"] = perf_counter() - start
    assert executable.is_file()
    if actor.startswith("mech"):
        assert sha(frontend_dir / "program.mecb") == runtime["image_sha256"]
        if actor == "mech":
            assert sha(encoder) == reference_pins["object_sha256"]
            assert sha(native / "metadata.json") == reference_pins["metadata_sha256"]
        else:
            assert sha(encoder) == llvm_sha
    result = {
        "actor": actor, "seconds": timings, "encoder_sha256": sha(encoder),
        "encoder_bytes": encoder.stat().st_size, "executable": str(executable),
        "executable_sha256": sha(executable),
    }
    (destination / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(actor, destination.parent.name, timings, flush=True)
    return result

if __name__ == "__main__":
    assert sha(roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
    assert sha(source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"
    results = work / sys.argv[1]
    results.mkdir()
    actors = ("mech", "mech-llvm", "roc", "libdeflate")
    warmup = [measure(actor, results / "warmup" / actor) for actor in actors]
    invocations = []
    # Every build occupies each position once, with every ordered pair
    # appearing together once. Fresh outputs are written to each cohort.
    orders = ((0,1,3,2), (1,2,0,3), (2,3,1,0), (3,0,2,1))
    for i, order in enumerate(orders, 1):
        cohort = results / f"order-{i}"
        invocations.extend(measure(actors[index], cohort / actors[index]) for index in order)
    profiles = [measure(actor, results / "profile" / actor, profile=True) for actor in actors]
    for result in [*warmup, *invocations, *profiles]:
        if not result["actor"].startswith("mech"):
            verify_peer(Path(result["executable"]))
    for actor in [a for a in actors if a.startswith("mech")]:
        mech_proof = results / "profile" / actor
        subprocess.run([sys.executable, "-B", q / "verify-worker.py", mech_proof / "compressor",
                        mech_proof / "native/metadata.json", f"matched-source-build-{actor}"], check=True)
    summaries = {}
    for actor in actors:
        runs = [row["seconds"] for row in invocations if row["actor"] == actor]
        summaries[actor] = {
            "medians": {k: statistics.median(r[k] for r in runs) for k in runs[0]},
            "total_range": [min(r["total"] for r in runs), max(r["total"] for r in runs)],
        }
    pins = {
        "platform": platform.platform(), "mech_source": sha(source_mech), "mech_frontend": sha(frontend),
        "mech_emitter": sha(emitter), "roc_app": sha(roc_app), "roc_compiler": sha(roc),
        "llvm_emitter": sha(llvm_emitter), "llvm_object": llvm_sha,
        "c_source": sha(source_c / "lib/deflate_compress.c"), "mimalloc": sha(mimalloc),
        "clang": subprocess.check_output([cc[0], "--version"], text=True).splitlines()[0],
        "roc_compiler_source": "d99c7b838fb05bb1047eb0c18d4d06d0f86f345d",
        "roc_compiler_build": "Zig 0.17.0 ReleaseFast",
        "mech_frontend_build": "Rust release opt-level 1, without PGO",
        "roc_encoder_source": "671077457ff7ef3a3d44caf715a3fc595addf379",
        "libdeflate_encoder_source": "92e6a0db9fa848d742f9eb286c92afc60f2c3dda",
    }
    print(summaries, flush=True)
    report = {"pins": pins, "summaries": summaries, "timed": invocations, "diagnostic": profiles}
    (results / "timing.json").write_text(json.dumps(report, indent=2) + "\n")
