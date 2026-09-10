#!/usr/bin/env python3
"""Run independent unittest cases in balanced subprocess shards."""

from __future__ import annotations

import argparse
import subprocess
import sys
import unittest
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Iterable


REPOSITORY = Path(__file__).resolve().parents[1]


def module_name(argument: str) -> str:
    path = Path(argument)
    if path.suffix == ".py":
        path = path.with_suffix("")
    return ".".join(part for part in path.parts if part not in ("", "."))


def iter_cases(suite: unittest.TestSuite) -> Iterable[unittest.TestCase]:
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from iter_cases(test)
        else:
            yield test


def partition(test_ids: list[str], jobs: int) -> list[list[str]]:
    shard_count = min(jobs, len(test_ids))
    shards = [[] for _ in range(shard_count)]
    for index, test_id in enumerate(test_ids):
        shards[index % shard_count].append(test_id)
    return shards


def select_shard(test_ids: list[str], shard_index: int, shard_count: int) -> list[str]:
    if shard_count < 1:
        raise ValueError("shard count must be positive")
    if shard_index < 0 or shard_index >= shard_count:
        raise ValueError("shard index is outside the shard count")
    return test_ids[shard_index::shard_count]


def run_shard(index: int, test_ids: list[str]) -> tuple[int, int, str]:
    completed = subprocess.run(
        [sys.executable, "-B", "-m", "unittest", *test_ids],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return index, completed.returncode, completed.stdout


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("tests", nargs="+")
    arguments = parser.parse_args()
    if arguments.jobs < 1:
        parser.error("--jobs must be positive")
    if arguments.shard_count < 1:
        parser.error("--shard-count must be positive")
    if arguments.shard_index < 0 or arguments.shard_index >= arguments.shard_count:
        parser.error("--shard-index must be within --shard-count")

    # Executing this file makes scripts/ sys.path[0]; add the repository so
    # dotted test IDs remain importable in both discovery and child processes.
    sys.path.insert(0, str(REPOSITORY))
    suite = unittest.defaultTestLoader.loadTestsFromNames(
        [module_name(argument) for argument in arguments.tests]
    )
    test_ids = select_shard(
        [test.id() for test in iter_cases(suite)],
        arguments.shard_index,
        arguments.shard_count,
    )
    if not test_ids:
        print("no unittest cases discovered", file=sys.stderr)
        return 2

    shards = partition(test_ids, arguments.jobs)
    failures = 0
    with ThreadPoolExecutor(max_workers=len(shards)) as executor:
        futures = {
            executor.submit(run_shard, index, shard): index
            for index, shard in enumerate(shards, start=1)
        }
        for future in as_completed(futures):
            index, returncode, output = future.result()
            print(f"--- unittest shard {index}/{len(shards)} ---")
            print(output, end="" if output.endswith("\n") else "\n")
            failures += returncode != 0

    if failures:
        print(f"{failures} unittest shard(s) failed", file=sys.stderr)
        return 1
    print(f"all {len(test_ids)} tests passed across {len(shards)} shards")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
