"""Check independent Mech source changes against a pinned linked control."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
checkout = q / "native-review-pr/checkout"
build_log = q / "source-frame-copy-tails/integrated-cargo.jsonl"
compiler = q / "host-opt-levels/opt3-cargo/release/compile_profile"
reference = study / "native/generic-control"
runtime = json.loads((reference / "runtime.json").read_text())
plan = json.loads((study / "native/plan.json").read_text())
env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, item):
    path.write_text(json.dumps(item, indent=2) + "\n")


def run(command, log, env):
    command = list(map(str, command))
    with log.open("wb") as stream:
        subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, env=env, check=True)
    return command


def build_with_expanded_parse_fuel(output):
    profiler = output / "compile_profile.rs"
    canonical = checkout / "benchmarks/mech-deflate/native-profile/compile_profile.rs"
    text = canonical.read_text()
    assert 'ParseConfig::default(),' in text
    text = text.replace('ParseConfig::default(),',
                        '{ let mut config = ParseConfig::default(); config.limits.fuel *= 2; config },')
    for relative in ("src/compiler_allocator.rs", "hosts/gpu/tests/fixtures/native/deflate_assembly.rs"):
        text = text.replace(f'"../../../{relative}"', f'"{checkout / relative}"')
    profiler.write_text(text)
    crates = {}
    for line in build_log.read_text().splitlines():
        if line.startswith("{"):
            row = json.loads(line)
            if row["reason"] == "compiler-artifact":
                for filename in row["filenames"]:
                    if filename.endswith(".rlib"):
                        crates[row["target"]["name"]] = filename
    executable = output / "compile_profile"
    dependencies = ("mech_gpu", "mech_engine", "mech_syntax", "mech_stdlib", "sha2", "serde_json", "mimalloc")
    command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "-C", "opt-level=3",
               "-C", "codegen-units=16", "-L", "dependency=" + str(compiler.parent / "deps"),
               "-o", executable, profiler]
    for name in dependencies:
        command.extend(("--extern", f"{name}={crates[name]}"))
    write(output / "profiler-command.json", run(command, output / "profiler-build.log", env))
    return executable


def qualify(source, label, compiler, output):
    out = output / label
    out.mkdir()
    commands = {"compile": run([compiler, source, out, "compact"], out / "compile.log", env | runtime["flags"])}
    link = runtime["link"]
    for file in ("program.o", "mech-worker"):
        assert sum(arg.count(str(reference / file)) for arg in link) == 1
        link = [arg.replace(str(reference / file), str(out / file)) for arg in link]
    commands["link"] = run(link, out / "link.log", env)
    result = {file: sha(out / file) for file in ("program.mecb", "program.o", "metadata.json", "mech-worker")}
    write(out / "runtime.json", runtime | {"link": link, "image_sha256": result["program.mecb"]})
    write(out / "identity.json", {"source_sha256": sha(source), "compiler_sha256": sha(compiler), **result})
    smoke = f"ordinary-mech-unmatched-{label}"
    commands["smokes"] = run([sys.executable, "-B", q / "verify-worker.py", out / "mech-worker",
                              out / "metadata.json", smoke], out / "smokes.log", env)
    receipt = json.loads((q / f"{smoke}-qualification.json").read_text())
    assert receipt["complete_smokes"] == 162
    sys.path.insert(0, str(q))
    from importlib import import_module
    macho = import_module("reachability-census")
    bytes_text = macho.load(out / "program.o")[1][macho.TEXT][2]
    write(out / "result.json", {"hashes": result, "commands": commands, "encoder_text_bytes": bytes_text, **receipt})
    print(label, bytes_text, receipt["complete_smokes"], result, flush=True)
    return result


assert sha(compiler) == plan["compiler_sha256"]
mode = sys.argv[1]
output = study / mode
output.mkdir()
if mode == "literal-once-proof":
    qualify(study / "literal-once.mec", "literal-once", compiler, output)
elif mode == "expanded-fuel-proof":
    compiler = build_with_expanded_parse_fuel(output)
    control = qualify(study / "control.mec", "expanded-control", compiler, output)
    assert control == json.loads((study / "native/generic-control.json").read_text())["hashes"]
    qualify(study / "no-inline-comment.mec", "expanded-literal-run", compiler, output)
else:
    raise SystemExit("expected literal-once-proof or expanded-fuel-proof")
