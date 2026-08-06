#!/usr/bin/env python3
"""Run the commands for one impact-classifier owner shard."""

from __future__ import annotations

import argparse
import shlex
import subprocess
from pathlib import Path

from ci_owners import DEFAULT_OWNER_CONFIG, REPOSITORY_ROOT, load_owners


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--owners", required=True, help="comma-separated owner names")
    parser.add_argument("--config", type=Path, default=DEFAULT_OWNER_CONFIG)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    owners = load_owners(args.config)
    selected = [name.strip() for name in args.owners.split(",") if name.strip()]
    if not selected:
        raise SystemExit("an owner shard must not be empty")

    for name in selected:
        if name not in owners:
            raise SystemExit(f"unknown CI owner: {name}")
        command = owners[name]["command"]
        if not command:
            raise SystemExit(f"CI owner has no test command: {name}")
        print(f"::group::{name}: {shlex.join(command)}", flush=True)
        try:
            subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)
        finally:
            print("::endgroup::", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
