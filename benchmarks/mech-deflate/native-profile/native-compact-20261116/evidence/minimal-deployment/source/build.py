#!/usr/bin/env python3
"""Link and strip a common raw-DEFLATE CLI around three pinned encoders."""
from argparse import ArgumentParser
from hashlib import file_digest
import json
from pathlib import Path
import shlex
from shutil import copy2
import subprocess

SOURCE = Path(__file__).resolve().parent
CLANG = ["/usr/bin/clang", "-arch", "arm64", "-mmacosx-version-min=15.0"]
ADAPTER = ["-O2", "-std=gnu11", "-Wall", "-Wextra", "-Werror"]
RUST = ["rustc", "+nightly-2026-03-03"]


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def validate_mech_abi(metadata_path):
    metadata = json.loads(metadata_path.read_bytes())
    byte = {"kind": {"float": False, "scalar_bytes": 1, "signed": False},
            "owners": [], "packed_align": 1, "packed_bytes": 1, "words": 1}
    view = {"kind": {"array_element": byte}, "owners": [5], "packed_align": 8,
            "packed_bytes": None, "words": 6}
    encoded_prefix = "mech-source-input-"
    inputs = [(row["name"], row["input_offset"], row["type"]) for row in metadata["inputs"]]
    expected = [(encoded_prefix + name.encode().hex(), offset, view)
                for name, offset in (("input", 0), ("output", 6))]
    shape = (metadata["symbol"], metadata["concrete_heap_context"], metadata["input_words"],
             metadata["output_words"], metadata["output_types"])
    if shape != ("mech_compact_closed_v2", True, 12, 6, [view]) or sorted(inputs) != sorted(expected):
        raise ValueError("Mech encoder does not implement the pinned compact U8 view ABI")


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("manifest", type=Path, help="JSON paths and SHA-256 for fixed inputs")
    cli.add_argument("output", type=Path, help="Fresh output directory")
    args = cli.parse_args()
    output = args.output.resolve()
    output.mkdir()
    manifest = json.loads(args.manifest.read_bytes())
    files = {}
    for name, record in manifest.items():
        path = (args.manifest.resolve().parent / record["path"]).resolve()
        if sha(path) != record["sha256"]:
            raise ValueError(f"wrong fixed input: {name} at {path}")
        files[name] = path
    validate_mech_abi(files["mech_metadata"])

    commands = {}

    def run(label, *argv):
        command = [str(argument) for argument in argv]
        commands[label] = command
        result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (output / f"{label}.log").write_bytes(result.stdout)
        if result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}): {result.stdout.decode(errors='replace')}")
        return result.stdout.decode()

    def c_object(name, include=None):
        obj = output / f"{name}.o"
        includes = () if include is None else ("-I", include)
        run(f"compile-{name}", *CLANG, *ADAPTER, *includes, "-c", SOURCE / f"{name}.c", "-o", obj)
        return obj

    main_object = c_object("main")
    roc_adapter = c_object("roc", files["roc_header"].parent)
    c_adapter = c_object("libdeflate", files["c_header"].parent)
    rust_source = output / "rust"
    rust_source.mkdir()
    for source in (SOURCE / "bridge.rs", SOURCE / "native_abi.rs", files["pinned_allocator"]):
        name = "pinned_allocator.rs" if source == files["pinned_allocator"] else source.name
        copy2(source, rust_source / name)
    mech_adapter = output / "libmech_deploy.a"
    rust_log = run(
        "compile-mech", *RUST, "--edition=2024", "--target=aarch64-apple-darwin",
        "--crate-name", "mech_deploy", "--crate-type=staticlib", "-D", "warnings",
        "-C", "opt-level=3", "-C", "debuginfo=0", "-C", "codegen-units=1", "-C", "panic=abort",
        "--print", "native-static-libs", "--extern", f"mech_gpu={files['mech_runtime']}",
        rust_source / "bridge.rs", "-o", mech_adapter,
    )
    rust_libraries, = (
        shlex.split(line.partition("native-static-libs: ")[2])
        for line in rust_log.splitlines() if "native-static-libs: " in line
    )
    backend_inputs = {
        "mech": [mech_adapter, files["mech_encoder"]],
        "roc": [roc_adapter, files["roc_encoder"]],
        "libdeflate": [c_adapter, files["c_encoder"], files["c_alloc"], files["c_arm"]],
    }
    results = {}
    for name, backend in backend_inputs.items():
        unstripped = output / name
        stripped = output / f"{name}-stripped"
        libraries = rust_libraries if name == "mech" else []
        run(
            f"link-{name}", *CLANG, "-Wl,-dead_strip", f"-Wl,-map,{output / (name + '.map')}",
            main_object, *backend, files["mimalloc"], *libraries, "-o", unstripped,
        )
        copy2(unstripped, stripped)
        run(f"strip-{name}", "/usr/bin/strip", "-S", "-x", stripped)
        results[name] = {}
        for variant, path in (("unstripped", unstripped), ("stripped", stripped)):
            results[name][variant] = {"bytes": path.stat().st_size, "sha256": sha(path)}
            run(f"sections-{name}-{variant}", "/usr/bin/size", "-m", path)
    run("clang-version", CLANG[0], "--version")
    run("rustc-version", *RUST, "--version", "--verbose")
    run("os-version", "/usr/bin/sw_vers")
    run("machine", "/usr/bin/uname", "-m")
    receipt = {
        "scope": "Raw level-12 DEFLATE deployment size with a common C file driver and final link",
        "inputs": {name: record["sha256"] for name, record in manifest.items()},
        "sources": {p.name: sha(p) for p in sorted(SOURCE.iterdir()) if p.suffix in (".c", ".h", ".rs", ".py")},
        "commands": commands,
        "executables": results,
    }
    (output / "build.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
