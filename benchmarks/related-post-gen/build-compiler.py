#!/usr/bin/env python3
"""Rebuild a coherent compiler closure from a recorded, pinned dependency plan.

The dependency plan records all retained library paths, hashes, features and
exact toolchain. Engine, Compute, Stdlib, Runtime and GPU rebuild together.
No network access, Cargo dependency resolution or benchmark source changes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def source_inventory(root):
    result = {}
    for parent, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in (".git", "target", "typecheck", "node_modules"))
        for name in sorted(files):
            path = Path(parent) / name
            if path.suffix == ".rs" or name == "Cargo.toml":
                result[str(path.relative_to(root))] = digest(path)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--opt-level", choices=("0", "1", "2", "3"), default="3")
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    plan = json.loads(args.plan.read_text())
    source, out = args.source.resolve(), args.out.resolve()
    original_source = plan["stages"][0]["cwd"]
    # Archived plans can live elsewhere than their original compiler outputs.
    # Derive the command prefix from its recorded output, not the JSON location.
    original_out = str(Path(plan["stages"][0]["command"][-1]).parent)
    out.mkdir(parents=True, exist_ok=True)
    for pin in plan["retained_dependencies"].values():
        if digest(pin["path"]) != pin["sha256"]:
            raise SystemExit(f"Retained library changed: {pin['path']}")
    inventory = source_inventory(source)
    source_hash = hashlib.sha256(json.dumps(inventory, sort_keys=True).encode()).hexdigest()
    (out / "source-inventory.json").write_text(json.dumps({"root": str(source), "sha256": source_hash, "files": inventory}, indent=2) + "\n")
    report = {"schema": "mech.related-post-gen.local-compiler-build.v1", "plan": str(args.plan.resolve()), "plan_sha256": digest(args.plan), "source": str(source), "source_sha256": source_hash, "opt_level": args.opt_level, "runtime_feature_omission": "watcher/notify/ignore absent in retained cache; unrelated to native evaluation", "stages": []}
    for stage in plan["stages"]:
        command = [arg.replace(original_source, str(source)).replace(original_out, str(out)) for arg in stage["command"]]
        command[command.index("opt-level=0")] = "opt-level=" + args.opt_level
        for i, arg in enumerate(command):
            if arg.startswith("metadata="):
                command[i] = f"metadata=related_{source_hash[:16]}_o{args.opt_level}_{stage['crate']}"
        if "--test" in command:
            command[command.index("--test")] = "--crate-type=rlib"
            command[-1] = str(out / f"lib{stage['crate']}.rlib")
        if Path(command[-1]).parent != out:
            raise SystemExit(f"Compiler output was not remapped into {out}")
        environment = {k: v.replace(original_source, str(source)) for k, v in stage["environment"].items()}
        receipt_path = out / f"{stage['crate']}.build.json"
        output = Path(command[-1])
        receipt = {"crate": stage["crate"], "command": command, "environment": environment, "source_sha256": source_hash}
        if args.resume and receipt_path.exists() and output.exists():
            prior = json.loads(receipt_path.read_text())
            if prior.get("command") == command and prior.get("source_sha256") == source_hash and prior.get("exit") == 0 and prior.get("library_sha256") == digest(output):
                report["stages"].append(prior)
                print(json.dumps({"crate": stage["crate"], "reused": True}), flush=True)
                continue
        started = time.monotonic()
        with (out / f"{stage['crate']}.stdout.log").open("w") as stdout, (out / f"{stage['crate']}.stderr.log").open("w") as stderr:
            result = subprocess.run(command, cwd=source, env={**os.environ, **environment}, stdout=stdout, stderr=stderr)
        receipt.update(exit=result.returncode, seconds=time.monotonic()-started)
        if result.returncode == 0:
            receipt["library_sha256"] = digest(output)
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
        report["stages"].append(receipt)
        (out / "build.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({k: receipt[k] for k in ("crate", "exit", "seconds")}), flush=True)
        if result.returncode:
            print((out / f"{stage['crate']}.stderr.log").read_text())
            raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
