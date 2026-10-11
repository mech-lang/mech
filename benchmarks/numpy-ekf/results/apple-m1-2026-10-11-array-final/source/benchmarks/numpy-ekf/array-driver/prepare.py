#!/usr/bin/env python3
"""Prepare the pinned paper backends with the generic array executor patch.

This does not compile or benchmark. Every numeric path uses the same snapshot;
the new executor is copied byte-for-byte from the production source. Generated
patches, source hashes, the manifest, and the dependency lock are retained.
"""

import argparse
import difflib
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

REVISION = "0d85319d60c460cdecb81e03c181d62ecfe1ef39"
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ARRAY_RELATIVE = Path("hosts/gpu/src/batched/array_cpu.rs")
MOD_RELATIVE = Path("hosts/gpu/src/batched/mod.rs")
FACTORY_RELATIVE = Path("hosts/gpu/src/compute_backends.rs")
ANCHOR = '#[cfg(feature = "jit")]\nmod jit;'
INSERTION = "mod array_cpu;\npub use array_cpu::*;\n\n"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--refresh", action="store_true", help="refresh only the exact array source and generated manifest")
    parser.add_argument("--lock", type=Path, help="reuse a retained dependency lockfile")
    args = parser.parse_args()
    destination = (args.destination or Path(tempfile.mkdtemp(prefix="mech-numpy-array-", dir="/private/tmp"))).resolve()
    destination.mkdir(parents=True, exist_ok=True)
    source, build = destination / "source", destination / "build"
    array_source = ROOT / ARRAY_RELATIVE
    if not array_source.is_file():
        parser.error(f"array production source does not exist: {array_source}")
    archive_command = ["git", "archive", REVISION, "Cargo.toml", "Cargo.lock", ".cargo", "src", "machines", "hosts/gpu", "tests/fixtures/resident-ekf"]
    if args.refresh:
        prior = json.loads((destination / "build-provenance.json").read_text())
        assert prior["revision"] == REVISION
        assert source.is_dir() and build.is_dir()
        original_mod = subprocess.check_output(["git", "show", f"{REVISION}:{MOD_RELATIVE}"], cwd=ROOT).decode()
    else:
        source.mkdir()
        build.mkdir()
        archive = subprocess.Popen(archive_command, cwd=ROOT, stdout=subprocess.PIPE)
        with tarfile.open(fileobj=archive.stdout, mode="r|") as entries:
            entries.extractall(source, filter="data")
        if archive.wait():
            raise RuntimeError("git archive failed")
        original_mod = (source / MOD_RELATIVE).read_text()
    assert original_mod.count(ANCHOR) == 1, "pinned module anchor changed"
    modified_mod = original_mod.replace(ANCHOR, INSERTION + ANCHOR)
    (source / MOD_RELATIVE).write_text(modified_mod)
    shutil.copy2(array_source, source / ARRAY_RELATIVE)
    patch = "".join(difflib.unified_diff(original_mod.splitlines(keepends=True), modified_mod.splitlines(keepends=True), fromfile=f"a/{MOD_RELATIVE}", tofile=f"b/{MOD_RELATIVE}"))
    original_factory = subprocess.check_output(["git", "show", f"{REVISION}:{FACTORY_RELATIVE}"], cwd=ROOT).decode()
    assert original_factory.count("BatchedSimdCpuSession") == 2
    assert original_factory.count("self.program.prepare_simd_cpu(&inputs)") == 1
    modified_factory = original_factory.replace("BatchedSimdCpuSession", "BatchedArrayCpuSession").replace("self.program.prepare_simd_cpu(&inputs)", "self.program.prepare_array_cpu(&inputs)")
    (source / FACTORY_RELATIVE).write_text(modified_factory)
    patch += "".join(difflib.unified_diff(original_factory.splitlines(keepends=True), modified_factory.splitlines(keepends=True), fromfile=f"a/{FACTORY_RELATIVE}", tofile=f"b/{FACTORY_RELATIVE}"))
    (destination / "array-integration.patch").write_text(patch)
    shutil.copy2(array_source, destination / "array_cpu.rs")
    manifest = (HERE / "Cargo.toml.in").read_text().replace("@MECH_SOURCE@", str(source)).replace("@DRIVER_SOURCE@", str(HERE / "src/main.rs")).replace("@ARRAY_TEST_SOURCE@", str(ROOT / "hosts/gpu/tests/batched_array_cpu.rs"))
    (build / "Cargo.toml").write_text(manifest)
    if args.lock:
        shutil.copy2(args.lock.resolve(), build / "Cargo.lock")
    metadata = {
        "revision": REVISION, "source": str(source), "manifest": str(build / "Cargo.toml"),
        "archive_command": archive_command,
        "checkout_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "array_source": str(array_source), "array_sha256": sha(array_source),
        "original_module_sha256": hashlib.sha256(original_mod.encode()).hexdigest(),
        "patched_module_sha256": sha(source / MOD_RELATIVE),
        "original_factory_sha256": hashlib.sha256(original_factory.encode()).hexdigest(),
        "patched_factory_sha256": sha(source / FACTORY_RELATIVE),
        "production_factory_sha256": sha(ROOT / FACTORY_RELATIVE),
        "integration_patch": str(destination / "array-integration.patch"), "integration_patch_sha256": sha(destination / "array-integration.patch"),
        "driver_source": str(HERE / "src/main.rs"), "driver_source_sha256": sha(HERE / "src/main.rs"),
        "fixture": str(HERE.parent / "mech-driver/ekf-numerical.mec"),
        "fixture_sha256": sha(HERE.parent / "mech-driver/ekf-numerical.mec"),
        "manifest_sha256": hashlib.sha256(manifest.encode()).hexdigest(),
    }
    (destination / "build-provenance.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
