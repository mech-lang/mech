#!/usr/bin/env python3
"""Generate the frozen D1 artifact, activation, and execution projections."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/d1-contract-generator/Cargo.toml"
OUTPUTS = {
    "artifact": ROOT / "tests/architecture/resident-activation/d1-artifact-v1.json",
    "activation": ROOT / "tests/architecture/resident-activation/d1-activation-v1.json",
    "execution": ROOT / "tests/architecture/resident-activation/d1-execution-v1.json",
}
EXPECTED_SHA256 = {
    "artifact": "8bba3ee55ecc7a5324853b8181a544bd3a37543383d975702d41fb0c284b9e24",
    "activation": "8e8f4c9260ae4b7e8fbfc5bde0998def7daf49fc7ff85da1810dc834cec64bda",
    "execution": "9f2dede0a0893af64b90e08432beb7bde9944f48338a51d9ecaba0f98ec13d2a",
}


def render(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode()


def generate_once() -> dict[str, object]:
    environment = dict(os.environ)
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target/d1-contract-generator")
    process = subprocess.run(
        ["cargo", "run", "--quiet", "--manifest-path", str(FIXTURE)],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    return json.loads(process.stdout)


def generated() -> dict[str, bytes]:
    runs = [generate_once() for _ in range(5)]
    canonical = json.dumps(runs[0], sort_keys=True, separators=(",", ":"))
    if any(json.dumps(run, sort_keys=True, separators=(",", ":")) != canonical for run in runs[1:]):
        raise RuntimeError("D1 projection generator is nondeterministic across fresh processes")
    return {name: render(runs[0][name]) for name in OUTPUTS}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    projections = generated()
    errors: list[str] = []
    for name, path in OUTPUTS.items():
        content = projections[name]
        digest = hashlib.sha256(content).hexdigest()
        if digest != EXPECTED_SHA256[name]:
            errors.append(f"{name} projection digest {digest} != pinned {EXPECTED_SHA256[name]}")
        if args.check:
            if not path.exists() or path.read_bytes() != content:
                errors.append(f"{path.relative_to(ROOT)} is not the mechanical D1 projection")
        else:
            path.write_bytes(content)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
