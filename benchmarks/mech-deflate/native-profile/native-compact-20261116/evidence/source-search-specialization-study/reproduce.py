"""Rebuild either independently pinned source arm with the normal compiler."""

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess


HERE = Path(__file__).resolve().parent


def sha(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("compiler", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--role", choices=("source-control", "source-split"), required=True)
    parser.add_argument("--checked", action="store_true")
    args = parser.parse_args()
    recipe = json.loads((HERE / "recipes.json").read_text())[args.role]
    compiler = args.compiler.resolve(strict=True)
    source = (HERE / recipe["source"]).resolve(strict=True)
    assert sha(source) == recipe["source_sha256"]
    flags = recipe["flags"].copy()
    profiles = []
    for entry in recipe["profiles"]:
        path = HERE / entry["file"]
        assert sha(path) == entry["sha256"]
        profiles.append(entry["symbol"] + "@" + str(path))
    flags["MECH_COMPACT_PHASE_BLOCK_LAYOUT"] = ";".join(profiles)
    if args.checked:
        flags["MECH_COMPACT_PAIR_REGISTER_SPILLS"] = "checked"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    command = [str(compiler), str(source), str(output / "frontend"), "compact"]
    with (output / "compile.log").open("w") as log:
        subprocess.run(command, check=True, stdout=log, stderr=log,
                       env={k: v for k, v in os.environ.items() if not k.startswith("MECH_")} | flags)
    actual = {name: sha(output / "frontend" / name) for name in recipe["artifacts"]}
    assert actual == recipe["artifacts"], (actual, recipe["artifacts"])
    result = {"role": args.role, "checked": args.checked, "command": command,
              "compiler_sha256": sha(compiler), "source_sha256": sha(source),
              "artifacts": actual, "exact_measured_artifacts": True,
              "scope": "Source through native object/full metadata/bytecode. Runtime qualification and final linking are separate."}
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
