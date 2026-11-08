#!/usr/bin/env python3
"""Build a benchmark adapter against a coherently rebuilt compiler closure."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import struct
import time


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def executable_validation_error(path, compiler_stderr):
    # rustc can return zero after rust-objcopy fails, leaving a damaged output.
    # This retained local closure targets thin AArch64 Mach-O executables.
    if "stripping debug info with" in compiler_stderr and "failed" in compiler_stderr:
        return "Rust reported failed executable postprocessing"
    try:
        with Path(path).open("rb") as executable:
            header = executable.read(32)
    except OSError as error:
        return f"Executable is unavailable: {error}"
    if len(header) != 32 or struct.unpack_from("<II", header) != (0xfeedfacf, 0x100000c):
        return "Expected a complete AArch64 Mach-O executable header"
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler-build", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--linked", action="store_true")
    args = parser.parse_args()
    build = json.loads(args.compiler_build.read_text())
    libraries = {}
    for stage in build["stages"]:
        if stage["exit"]:
            raise SystemExit("Compiler build failed")
        path = Path(stage["command"][-1])
        if sha(path) != stage["library_sha256"]:
            raise SystemExit(f"Compiler library changed: {path}")
        libraries[stage["crate"]] = {"path": str(path), "sha256": sha(path)}
    gpu = next(s for s in build["stages"] if s["crate"] == "mech_gpu")["command"]
    command = [gpu[0], "--edition=2024", "-C", "opt-level=3", "-C", "embed-bitcode=no", "-C", "debug-assertions=off", "-C", "overflow-checks=off", "-C", "strip=debuginfo"]
    externs = {}
    for i, arg in enumerate(gpu):
        if arg == "--extern":
            name, path = gpu[i+1].split("=", 1)
            externs[name] = path
        if arg == "-L":
            command += ["-L", gpu[i+1]]
    for name in ("mech_core", "mech_engine", "mech_syntax", "mech_gpu", "mech_stdlib", "serde", "serde_json"):
        path = libraries[name]["path"] if name in libraries else externs[name]
        libraries[name] = {"path": path, "sha256": sha(path)}
        command += ["--extern", f"{name}={path}"]
    # Derived runtime/host feature builds can place one verified dependency
    # outside the original compiler output directories. Make every recorded
    # library discoverable, including transitive dependencies of mech_gpu.
    for parent in sorted({str(Path(pin["path"]).parent) for pin in libraries.values()}):
        search = f"dependency={parent}"
        if not any(arg == "-L" and command[i + 1] == search
                   for i, arg in enumerate(command[:-1])):
            command += ["-L", search]
    bundle = {}
    if args.bundle:
        for name in ("source.mec", "artifact.mecb", "object.o", "descriptor.json", "linked.rs", "metadata.json"):
            path = args.bundle.resolve() / name
            bundle[name] = {"path": str(path), "sha256": sha(path)}
        for name in ("frame-summary.json", "image-binding.bin", "catalog.rs", "catalog-profile.json",
                     "native-metadata.bin", "native-metadata-binding.bin"):
            path = args.bundle.resolve() / name
            if path.exists():
                bundle[path.name] = {"path": str(path), "sha256": sha(path)}
    if args.linked:
        if not args.bundle:
            raise SystemExit("--linked needs --bundle")
        command += ["--cfg", "linked", "-C", f"link-arg={args.bundle.resolve() / 'object.o'}"]
    command += [str(args.source.resolve()), "-o", str(args.out.resolve())]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    result = subprocess.run(command, capture_output=True, text=True, env={**os.environ, "RUSTC_BOOTSTRAP": "1"})
    artifact_error = executable_validation_error(args.out, result.stderr) if result.returncode == 0 else None
    exit_code = result.returncode or int(artifact_error is not None)
    receipt = {"schema": "mech.related-post-gen.local-host-build.v1", "compiler_build": str(args.compiler_build.resolve()), "compiler_build_sha256": sha(args.compiler_build), "compiler_source_sha256": build["source_sha256"], "source_path": str(args.source.resolve()), "source_sha256": sha(args.source), "command": command, "libraries": libraries, "bundle": bundle, "seconds": time.monotonic()-started, "exit": exit_code, "compiler_exit": result.returncode, "executable_validation_error": artifact_error, "stdout": result.stdout, "stderr": result.stderr, "allocator": "Rust default system allocator"}
    if args.out.is_file():
        receipt["executable_sha256"] = sha(args.out)
    args.out.with_suffix(args.out.suffix + ".build.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(result.stdout, end="")
    print(result.stderr, end="")
    if artifact_error:
        print(artifact_error, flush=True)
    print(json.dumps({k: receipt[k] for k in ("seconds", "exit")}), flush=True)
    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
