#!/usr/bin/env python3
"""Execute a generated application against the maintained small-stream corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import time
import zlib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def payload(snapshot):
    prefix = b"mech-snapshot-v1\0"
    assert snapshot.startswith(prefix)
    at = len(prefix) + 32
    shape_bytes, = struct.unpack_from("<Q", snapshot, at)
    at += 8 + shape_bytes
    count, = struct.unpack_from("<Q", snapshot, at)
    raw = snapshot[at + 8:]
    assert len(raw) == count
    return raw


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("application", type=Path)
    parser.add_argument("binary", type=Path)
    parser.add_argument("launch", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    for key in ["application", "binary", "launch"]:
        setattr(args, key, getattr(args, key).resolve(strict=True))
    args.evidence.mkdir(parents=True, exist_ok=False)
    cases = json.loads((args.application / "cases.json").read_text())
    receipts = []
    try:
        for case in cases:
            command = [str(args.binary), "--policy", str(args.launch), "--", str(args.application / case["input"])]
            started = time.monotonic()
            result = subprocess.run(command, capture_output=True, timeout=600)
            seconds = time.monotonic() - started
            (args.evidence / (case["name"] + ".stdout")).write_bytes(result.stdout)
            (args.evidence / (case["name"] + ".stderr")).write_bytes(result.stderr)
            receipt = {"name": case["name"], "command": command, "exit": result.returncode, "process_seconds": seconds,
                       "input_bytes": case["input_bytes"], "exact_bytes": False, "independent_inflation": False}
            receipts.append(receipt)
            result.check_returncode()
            lines = result.stdout.decode().splitlines()
            assert len(lines) == case["output_lines"]
            assert lines[-1] == case["last_snapshot_hex"]
            raw = payload(bytes.fromhex(lines[-1]))
            assert raw == bytes(case["raw"])
            assert raw[:len(case["prefix"])] == bytes(case["prefix"])
            receipt["exact_bytes"] = True
            assert zlib.decompress(raw[len(case["prefix"]):], -15) == bytes(case["plain"])
            receipt["independent_inflation"] = True
            print(json.dumps(receipt), flush=True)
    finally:
        (args.evidence / "results.json").write_text(json.dumps({"binary_sha256": digest(args.binary), "launch_sha256": digest(args.launch),
            "cases_sha256": digest(args.application / "cases.json"), "passed": len(receipts) == len(cases) and all(r["exact_bytes"] and r["independent_inflation"] for r in receipts),
            "cases": receipts, "timing_boundary": "whole process including image restore, input import, native execution, canonical output export and cleanup; qualification, not compression-only benchmark"}, indent=2) + "\n")


if __name__ == "__main__":
    main()
