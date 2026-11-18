"""Reproduce the rejected experiment's exact native object and metadata."""

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
packet = here.parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("emitter", type=Path)
parser.add_argument("image", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()


def read(file):
    return json.loads(file.read_text())


def digest(file):
    return sha256(file.read_bytes()).hexdigest()


effective = read(here / "artifacts/candidate-effective.json")
report = read(here / "report.json")
recipe = read(packet / "recipe.json")
assert digest(args.image) == effective["image_sha256"] == report["source_bytecode_sha256"]
flags = effective["flags"].copy()
profiles = {entry["symbol"]: packet / entry["path"] for entry in recipe["profiles"]}
phase_entries = flags["MECH_COMPACT_PHASE_BLOCK_LAYOUT"].split(";")
symbols = [entry.split("@", 1)[0] for entry in phase_entries]
assert set(symbols) == profiles.keys()
flags["MECH_COMPACT_PHASE_BLOCK_LAYOUT"] = ";".join(f"{name}@{profiles[name]}" for name in symbols)
flags["MECH_COMPACT_ROOT_TAIL_PROFILE"] = str(packet / recipe["zero_branches_tail_profile"])
assert flags["MECH_ORDERED_VECTOR_CURSORS"] == "1"
environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")} | flags
environment["PATH"] = "/usr/bin:/bin"
output = args.output.resolve()
output.mkdir(parents=True)
subprocess.run([args.emitter.resolve(strict=True), args.image.resolve(), output], env=environment, check=True)
expected = report["actors"]["candidate"]["identity"]
for field in ("program.o", "metadata.json"):
    assert digest(output / field) == expected[field], f"{field} differs from the recorded experiment"
print("Exact candidate object and metadata reproduced.")
