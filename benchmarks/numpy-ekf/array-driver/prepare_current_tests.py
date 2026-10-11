#!/usr/bin/env python3
"""Generate a minimal manifest for the actual checkout's array integration tests.

This shares the test feature set with the pinned benchmark driver, while using
current production paths and the current checkout's supported JIT-only feature.
No compilation is started here.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path)
    args = parser.parse_args()
    destination = (args.destination or Path(tempfile.mkdtemp(prefix="mech-array-current-tests-", dir="/private/tmp"))).resolve()
    destination.mkdir(parents=True, exist_ok=True)
    manifest = (HERE / "Cargo.toml.in").read_text()
    manifest = re.sub(r'\[\[bin\]\]\nname = "numpy-ekf-array-driver"\npath = "@DRIVER_SOURCE@"\n\n', "", manifest)
    assert "@DRIVER_SOURCE@" not in manifest
    manifest = manifest.replace("@MECH_SOURCE@", str(ROOT)).replace("@ARRAY_TEST_SOURCE@", str(ROOT / "hosts/gpu/tests/batched_array_cpu.rs"))
    manifest = manifest.replace('features = ["jit", "aot"]', 'features = ["jit"]')
    (destination / "Cargo.toml").write_text(manifest)
    metadata = {
        "checkout": str(ROOT), "manifest": str(destination / "Cargo.toml"),
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "tracked_changes": subprocess.check_output(["git", "diff", "--name-only"], cwd=ROOT, text=True).splitlines(),
        "manifest_sha256": hashlib.sha256(manifest.encode()).hexdigest(),
        "source_sha256": {relative: hashlib.sha256((ROOT / relative).read_bytes()).hexdigest() for relative in (
            "hosts/gpu/src/batched/array_cpu.rs", "hosts/gpu/src/batched/mod.rs", "hosts/gpu/src/compute_backends.rs", "hosts/gpu/tests/batched_array_cpu.rs")},
    }
    (destination / "test-provenance.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
