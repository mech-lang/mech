"""Verify native identities and every raw runtime observation."""

import ast
from collections import Counter
import gzip
from hashlib import sha256
from itertools import permutations
import json
import math
from pathlib import Path
import statistics
import struct
import tarfile
import tempfile


HERE = Path(__file__).resolve().parent
VALUE_VALIDATOR = None


def read(path):
    return json.loads(Path(path).read_text())


def sha(data):
    return sha256(data).hexdigest()


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-14), (a, b)


def matcher_bytes(path, receipt):
    image = path.read_bytes()
    assert struct.unpack_from("<I", image)[0] == 0xFEEDFACF
    at = 32
    text = None
    for _ in range(struct.unpack_from("<I", image, 16)[0]):
        kind, size = struct.unpack_from("<II", image, at)
        if kind == 0x19:
            for index in range(struct.unpack_from("<I", image, at + 64)[0]):
                section = at + 72 + index * 80
                if image[section:section + 16].rstrip(b"\0") == b"__text":
                    address, extent, offset = struct.unpack_from("<QQI", image, section + 32)
                    text = (address, image[offset:offset + extent])
        at += size
    assert text is not None
    atom = receipt["layout"]["symbols"]["_mech_compact_numeric_475_vector"]
    assert not atom["relocations"]
    base, payload = text
    start = atom["start"] - base
    assert 0 <= start < len(payload)
    assert sha(payload[start:start + atom["extent"]]) == atom["sha256"]


def runtime(root, runs, expected_cases, expected_blocks, required=None):
    checked = 0
    zero_duration = 0
    outcomes = []
    for run in runs:
        folder = root / Path(run["run"]).name
        assert sha((folder / "summary.json").read_bytes()) == run["summary_sha256"]
        rows = read(folder / "summary.json")
        assert rows == run["results"] and len(rows) == expected_cases
        for row in rows:
            blocks = json.loads(gzip.decompress(
                (folder / row["case"] / "observations.json.gz").read_bytes()))
            assert len(blocks) == expected_blocks
            actors = set(blocks[0]["order"])
            assert len(actors) == (5 if expected_blocks == 120 else 4)
            order_counts = Counter(tuple(b["order"]) for b in blocks)
            assert set(order_counts) == set(permutations(sorted(actors)))
            assert set(order_counts.values()) == ({1} if len(actors) == 5 else {4})
            logs = {peer: {actor: [] for actor in actors} for peer in ("roc", "libdeflate")}
            for block in blocks:
                assert set(block["observations"]) == actors
                means = {}
                for actor, observations in block["observations"].items():
                    assert observations
                    for value in observations:
                        assert value["case"] == row["case"] and value["seconds"] >= 0
                        zero_duration += value["seconds"] == 0
                        assert all(value[key] for key in (
                            "independent_inflation", "input_alias_unchanged", "borrowed_output_alias_unchanged"))
                        if actor != "libdeflate":
                            assert value["exact_reference"]
                        if actor.startswith("mech-"):
                            assert value["nonreturned_private_bytes_at_return"] == 0
                        if VALUE_VALIDATOR is not None:
                            VALUE_VALIDATOR(actor, value, row)
                        checked += 1
                    means[actor] = statistics.mean(v["seconds"] for v in observations)
                    assert means[actor] > 0
                    close(means[actor], block["means"][actor])
                for peer in logs:
                    for actor in actors:
                        logs[peer][actor].append(math.log(means[actor] / means[peer]))
            for peer in logs:
                for actor in actors:
                    close(math.exp(statistics.mean(logs[peer][actor])), row["paired_x_" + peer][actor])
        if required:
            peers = {}
            for peer, names in required.items():
                wins = {row["case"] for row in rows if row["paired_x_" + peer]["mech-experiment"] < 1}
                peers[peer] = {"wins": len(wins), "required_missing": sorted(set(names) - wins),
                               "passed": set(names).issubset(wins)}
            assert peers == run["peers"] and all(p["passed"] for p in peers.values())
            outcomes.append(peers)
    return checked, outcomes, zero_duration
