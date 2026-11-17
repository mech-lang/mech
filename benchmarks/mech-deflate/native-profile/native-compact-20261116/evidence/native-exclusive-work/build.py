"""Build old or shared diagnostic compile_profile in the private checkout."""

import argparse
import json
import os
from pathlib import Path
import subprocess

from replay import digest


HERE = Path(__file__).resolve().parent
SOURCE = HERE / "source"
RECIPE = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/backend-work-hardened/compiler-plan.json")


def build(role, attempt):
    plan = json.loads(RECIPE.read_text())
    command = list(plan["command"])
    command[command.index("--manifest-path") + 1] = str(SOURCE / "benchmarks/mech-deflate/native-profile/Cargo.toml")
    command[command.index("--target-dir") + 1] = str(HERE / "target")
    owner = HERE / "shared-facts-only.patch"
    check = ["git", "apply", "--check", str(owner)]
    if role == "old":
        subprocess.run(check, cwd=SOURCE, check=True)
    elif subprocess.run(check, cwd=SOURCE, capture_output=True).returncode == 0:
        subprocess.run(["git", "apply", str(owner)], cwd=SOURCE, check=True)
    else:
        subprocess.run(["git", "apply", "--reverse", "--check", str(owner)], cwd=SOURCE, check=True)
    directory = HERE / f"build-{role}-{attempt}"
    directory.mkdir()
    environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    profile = plan["effective_profile"]
    record = {"role": role, "command": command, "cwd": str(SOURCE), "profile": profile,
              "recipe_sha256": digest(RECIPE), "trace_patch_sha256": digest(HERE / "trace.patch"),
              "owner_source_sha256": digest(SOURCE / "hosts/gpu/src/native_artifact/program/codegen/numeric_compact.rs")}
    (directory / "command.json").write_text(json.dumps(record, indent=2) + "\n")
    with (directory / "cargo.log").open("wb") as stream:
        subprocess.run(command, cwd=SOURCE, env=environment | profile, stdout=stream, stderr=subprocess.STDOUT, check=True)
    built = HERE / "target/release/compile_profile"
    compiler = directory / "compile_profile"
    subprocess.run(["/bin/cp", "-c", str(built), str(compiler)], check=True)
    (directory / "compiler.sha256").write_text(digest(compiler) + "\n")
    print(compiler, flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("role", choices=("old", "shared"))
    parser.add_argument("attempt", type=int)
    options = parser.parse_args()
    build(options.role, options.attempt)
