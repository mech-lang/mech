"""Verify the source compiler and the maintained rebuild against the timed worker."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys
from time import perf_counter

study = Path(__file__).resolve().parent
q = study.parent
checkout = q / "native-review-pr/checkout"
package = checkout / "benchmarks/mech-deflate/native-profile"
packet = package / "native-compact-20261116"
target = q / "host-opt-levels/opt3-cargo"
emitter = target / "release/emit_compact_size"
bytecode = q / "direct-source-20261117/rebuild-direct/frontend/program.mecb"
baseline = q / "stale-cache-accounting/pinned-smaller"
aligned = q / "branch-on-zero-aligned-native/final-candidate"
tail = packet / "profiles/checked-root-zero-branches.json"
compiled = study / "source-aligned"
result = {}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def run(name, arguments, flags=None, expect_failure=False):
    command = list(map(str, arguments))
    env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    env.update((flags or {}) | {"CARGO_NET_OFFLINE": "true"})
    log = study / f"{name}.log"
    start = perf_counter()
    with log.open("wb") as stream:
        proc = subprocess.run(command, env=env, stdout=stream, stderr=subprocess.STDOUT)
    assert (proc.returncode != 0) == expect_failure, log.read_text()[-1800:]
    result[name] = {"command": command, "flag_overrides": flags, "elapsed_seconds": perf_counter() - start,
                    "returncode": proc.returncode, "log_sha256": sha(log)}


profile = {
    "CARGO_BUILD_JOBS": "1", "CARGO_PROFILE_RELEASE_OPT_LEVEL": "3",
    "CARGO_PROFILE_RELEASE_DEBUG": "false", "CARGO_PROFILE_RELEASE_LTO": "false",
    "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "16",
}
run("compiler-construction", [
    "cargo", "+nightly-2026-03-03", "build", "--locked", "--release",
    "--config", "profile.release.package.libmimalloc-sys.opt-level=1",
    "--manifest-path", package / "Cargo.toml", "--target-dir", target,
    "--bin", "compile_profile", "--bin", "emit_compact_size", "--bin", "native_compact_frontend",
    "--message-format=json",
], profile)
run("pinned-worker-rebuild", [
    sys.executable, "-B", q / "probe-literal-reuse.py", "--native-reference", baseline,
    "--emitter", emitter, compiled, bytecode, "MECH_COMPACT_INLINE_SINGLE_CALLS=preselected",
    "MECH_COMPACT_BRANCH_ON_ZERO=1", f"MECH_COMPACT_ROOT_TAIL_PROFILE={tail}",
])
for name in ("program.o", "metadata.json", "mech-worker"):
    assert (compiled / name).read_bytes() == (aligned / name).read_bytes(), name
result["timed_worker_identities"] = {name: sha(compiled / name) for name in ("program.o", "metadata.json", "mech-worker")}

variant = "fresh-owner-counts-cold-tails-zero-branches"
allocator = next(
    arg.split("=", 1)[1] for arg in json.loads((aligned / "runtime.json").read_text())["link"]
    if arg.endswith("/mimalloc.o")
)
outputs = {}
for mode in ("direct", "split", "existing-paired"):
    directory = study / f"rebuild-{mode}"
    mode_variant = "fresh-owner-counts-cold-tails-paired-spills" if mode == "existing-paired" else variant
    command = [sys.executable, "-B", packet / "rebuild.py", directory, "--target-dir", target,
               "--compiler-opt-level", "3", "--variant", mode_variant, "--preselect-root-inlining"]
    if mode != "split":
        command.append("--direct-source")
    if mode != "existing-paired":
        command.extend(("--mimalloc-object", allocator))
    run(f"rebuild-{mode}", command)
    record = json.loads((directory / "rebuild.json").read_text())
    assert record["full_integrated_source_identity"]
    outputs[mode] = record
assert outputs["direct"]["verified_exact_calls"] == outputs["split"]["verified_exact_calls"] == 162
assert outputs["direct"]["worker_sha256"] == outputs["split"]["worker_sha256"]
result["maintained_rebuilds"] = outputs

bad = study / "invalid-tail-profile.json"
layout = json.loads(tail.read_text())
bad.write_text(json.dumps(layout | {"code_bytes": layout["code_bytes"] + 4}) + "\n")
flagged = json.loads((compiled / "runtime.json").read_text())["flags"]
invalid = study / "invalid-tail-output"
run("stale-tail-profile-rejection", [emitter, bytecode, invalid],
    flagged | {"MECH_COMPACT_ROOT_TAIL_PROFILE": str(bad), "PATH": "/usr/bin:/bin"},
    expect_failure=True)
assert "object tail profile does not match the checked root" in (study / "stale-tail-profile-rejection.log").read_text()
assert not (invalid / "program.o").exists()
(study / "source-reproduction.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result["timed_worker_identities"], indent=2), flush=True)
