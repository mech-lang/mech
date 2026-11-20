"""Build separate diagnostics around the exact selected encoder and pinned C.

All external file paths and hashes are supplied in a saved local config. The
shipping worker, codec object, original runtime and compiler are read only.
"""
import argparse
from hashlib import sha256
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    here, out = Path(__file__).resolve().parent, args.output.resolve()
    config = json.loads(args.config.read_text())
    assert all(digest(path) == pin for path, pin in config["inputs_sha256"].items())
    out.mkdir(parents=True, exist_ok=False)
    (out / "config.json").write_text(json.dumps(config, indent=2) + "\n")
    runtime = Path(config["runtime_source"])
    staged = out / "compact_runtime"
    staged.mkdir()
    for name in ("mod.rs", "terminal_appends.rs", "zero_pages.rs"):
        shutil.copyfile(runtime / name, staged / name)
    spec = importlib.util.spec_from_file_location("setup_instrument", here / "instrument.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    (staged / "mod.rs").write_text(module.instrument((staged / "mod.rs").read_text()))
    shutil.copyfile(here / "probe.rs", staged / "setup_probe.rs")
    shutil.copyfile(here / "worker.rs", out / "worker.rs")
    shutil.copyfile(config["allocator_source"], out / "pinned_allocator.rs")
    (out / "runtime.rs").write_text('pub mod native_artifact { #[path = ' +
        json.dumps(str(staged / "mod.rs")) + '] pub mod compact_runtime; }\n')
    builds = []

    def run(command, label):
        with (out / (label + ".log")).open("w") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        builds.append({"label": label, "command": command,
                       "log_sha256": digest(out / (label + ".log"))})

    run(["/usr/bin/clang", "-arch", "arm64", "-std=c11", "-O2", "-Wall", "-Werror",
         "-c", str(here / "probe.c"), "-o", str(out / "probe.o")], "probe-c")
    roles = {}
    base_link = config["selected_link"]
    for role in ("mech-control", "mech-counts", "mech-clocks"):
        if role == "mech-control":
            library = config["runtime_library"]
        else:
            library = str(out / ("lib" + role.replace("-", "_") + ".rlib"))
            command = ["rustc", config["toolchain"], "--edition=2024", "--crate-name", "mech_gpu",
                "--crate-type=rlib", "-C", "opt-level=3", "--check-cfg", "cfg(setup_probe_timing)",
                str(out / "runtime.rs"), "-o", library]
            if role == "mech-clocks":
                command += ["--cfg", "setup_probe_timing"]
            run(command, role + "-runtime")
        command = list(base_link)
        command[command.index(config["worker_source"])] = str(out / "worker.rs")
        command[command.index("-o") + 1] = str(out / role)
        command[command.index("mech_gpu=" + config["runtime_library"])] = "mech_gpu=" + library
        command += ["--check-cfg", "cfg(setup_probe)"]
        if role != "mech-control":
            command += ["--cfg", "setup_probe"]
        for path in [str(out / "probe.o"), *config["c_objects"]]:
            command += ["-C", "link-arg=" + path]
        run(command, role + "-worker")
        roles[role] = {"worker": str(out / role), "worker_sha256": digest(out / role),
                       "runtime_library": library, "runtime_sha256": digest(library)}
    manifest = {"config_sha256": digest(args.config), "builds": builds, "roles": roles,
        "script_inputs": {p.name: digest(p) for p in here.iterdir() if p.suffix in (".py", ".rs", ".c")},
        "generated_runtime_sources": {p.name: digest(p) for p in staged.iterdir()},
        "selected_object_sha256": digest(config["selected_object"]),
        "scope": "Separate diagnostic hosts. Exact selected Mech and pinned C encoder objects; no shipping worker replacement or performance qualification.",
        "complete": True}
    (out / "build.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"complete": True, "roles": roles}, indent=2), flush=True)


if __name__ == "__main__":
    main()
