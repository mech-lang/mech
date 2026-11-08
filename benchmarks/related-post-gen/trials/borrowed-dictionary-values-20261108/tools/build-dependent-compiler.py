#!/usr/bin/env python3
"""Rebuild changed Mech crates and their dependents with verified source reuse."""
import argparse
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--repair-failed-stages", action="store_true",
                        help="Reuse verified successful dependencies from an interrupted build; failed stages always rebuild.")
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("build_helpers", Path(__file__).with_name("build-compiler.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    base = json.loads(args.build.read_text())
    old_root = Path(base["source"])
    source, out = args.source.resolve(), args.out.resolve()
    old_files = json.loads((args.build.parent / "source-inventory.json").read_text())["files"]
    files = helper.source_inventory(source)
    source_hash = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    out.mkdir(parents=True, exist_ok=True)
    (out / "source-inventory.json").write_text(json.dumps(dict(root=str(source), sha256=source_hash, files=files), indent=2) + "\n")
    stages = base["stages"]
    names = {stage["crate"] for stage in stages}
    assert len(names) == len(stages)
    for stage in stages:
        if stage["exit"] != 0:
            assert args.repair_failed_stages, stage["crate"]
        else:
            assert sha(stage["command"][-1]) == stage["library_sha256"], stage["crate"]
    report = dict(schema="mech.related-post-gen.dependent-compiler-build.v1", source=str(source), source_sha256=source_hash, opt_level=base["opt_level"], base_build=str(args.build.resolve()), base_build_sha256=sha(args.build), stages=[], reuse_policy="A crate reuses its library only when every recorded Rust/manifest source under its manifest directory matches and none of its Mech dependencies rebuilt. All reused and rebuilt library hashes are verified.")
    libraries, rebuilt = {}, set()
    for prior in stages:
        crate = prior["crate"]
        manifest = Path(prior["environment"]["CARGO_MANIFEST_DIR"])
        # A verified reused stage can retain its original checkout path across
        # several dependent builds. Resolve its component against the recorded
        # inventory rather than assuming every library came from old_root.
        candidates = [Path(*manifest.parts[i:]) for i in range(1, len(manifest.parts))
                      if str(Path(*manifest.parts[i:]) / "Cargo.toml") in old_files]
        assert len(candidates) == 1, (crate, manifest, candidates)
        component_path = candidates[0]
        prefix = str(component_path) + "/"
        stage_root = manifest
        for _ in component_path.parts:
            stage_root = stage_root.parent
        old_component = {k: v for k, v in old_files.items() if k.startswith(prefix)}
        component = {k: v for k, v in files.items() if k.startswith(prefix)}
        assert old_component and component, (crate, prefix)
        dependencies = {prior["command"][i + 1].split("=", 1)[0] for i, arg in enumerate(prior["command"]) if arg == "--extern"} & names
        assert dependencies <= libraries.keys(), (crate, dependencies)
        changed = prior["exit"] != 0 or component != old_component or bool(dependencies & rebuilt)
        if not changed:
            stage = copy.deepcopy(prior)
            stage.update(reused=True, reused_component_source_sha256=hashlib.sha256(json.dumps(component, sort_keys=True).encode()).hexdigest(), reused_dependencies_unchanged=True)
        else:
            rebuilt.add(crate)
            command = [arg.replace(str(stage_root), str(source)) for arg in prior["command"]]
            command[-1] = str(out / f"lib{crate}.rlib")
            search_roots = sorted({str(out), *(str(Path(lib["path"]).parent) for lib in libraries.values())})
            command[1:1] = [arg for root in search_roots for arg in ["-L", "dependency=" + root]]
            for i, arg in enumerate(command):
                if arg == "--extern":
                    name = command[i + 1].split("=", 1)[0]
                    if name in libraries:
                        command[i + 1] = name + "=" + libraries[name]["path"]
                elif arg.startswith("metadata="):
                    command[i] = f"metadata=related_dependent_{source_hash[:16]}_{crate}"
            environment = {k: v.replace(str(stage_root), str(source)) for k, v in prior["environment"].items()}
            receipt = out / f"{crate}.build.json"
            stage = dict(crate=crate, command=command, environment=environment, source_sha256=source_hash, reused=False)
            if args.resume and receipt.exists():
                previous = json.loads(receipt.read_text())
                if previous.get("command") == command and previous.get("source_sha256") == source_hash and previous.get("exit") == 0 and Path(command[-1]).exists() and sha(command[-1]) == previous["library_sha256"]:
                    stage = previous
            if "exit" not in stage:
                start = time.monotonic()
                with (out / f"{crate}.stdout.log").open("w") as stdout, (out / f"{crate}.stderr.log").open("w") as stderr:
                    result = subprocess.run(command, cwd=source, env={**os.environ, **environment}, stdout=stdout, stderr=stderr)
                stage.update(exit=result.returncode, seconds=time.monotonic() - start)
                if result.returncode == 0:
                    stage["library_sha256"] = sha(command[-1])
                receipt.write_text(json.dumps(stage, indent=2) + "\n")
        report["stages"].append(stage)
        (out / "build.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(dict(crate=crate, reused=not changed, exit=stage["exit"], seconds=stage.get("seconds"))), flush=True)
        if stage["exit"]:
            print((out / f"{crate}.stderr.log").read_text())
            raise SystemExit(stage["exit"])
        libraries[crate] = dict(path=stage["command"][-1], sha256=stage["library_sha256"])
    assert all(sha(value["path"]) == value["sha256"] for value in libraries.values())


if __name__ == "__main__":
    main()
