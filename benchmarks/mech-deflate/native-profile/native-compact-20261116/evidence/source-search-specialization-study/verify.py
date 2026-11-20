"""Verify source, native identities, all raw runtime observations and build records."""

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


def main():
    summary = read(HERE / "summary.json")
    assert not summary["selected_default_changed"]
    assert summary["C_next_milestone"] == 64 and summary["C_ultimate_target"] == 81
    generator = ast.parse((HERE / "generate.py").read_text())
    values = {node.targets[0].id: ast.literal_eval(node.value)
              for node in generator.body if isinstance(node, ast.Assign)}
    original = (HERE / "../../program.mec").read_bytes()
    assert sha(original) == values["ORIGINAL_SHA256"]
    assert original.decode().count(values["BEFORE"]) == 1
    assert (HERE / "program.mec").read_text() == original.decode().replace(values["BEFORE"], values["AFTER"])
    manifests = read(HERE / "archive-files.json")
    assert manifests == summary["archives"]
    with tempfile.TemporaryDirectory(prefix="mech-source-search-") as temp:
        root = Path(temp)
        for name, pin in manifests.items():
            path = HERE / name
            assert sha(path.read_bytes()) == pin["sha256"]
            with tarfile.open(path) as tar:
                assert len(tar.getmembers()) == len(pin["members"])
                assert {m.name for m in tar.getmembers()} == set(pin["members"])
                for member in tar.getmembers():
                    assert member.isfile() and not member.name.startswith("/")
                    assert ".." not in Path(member.name).parts
                    assert sha(tar.extractfile(member).read()) == pin["members"][member.name]
                tar.extractall(root, filter="data")
        recipes = read(HERE / "recipes.json")
        for key, role in (("source-control", "source-search-control-profiled"),
                          ("source-split", "source-search-split-profiled")):
            recipe = recipes[key]
            assert sha((HERE / recipe["source"]).read_bytes()) == recipe["source_sha256"]
            for profile in recipe["profiles"]:
                assert sha((HERE / profile["file"]).read_bytes()) == profile["sha256"]
            receipt = read(root / role / "receipt.json")
            assert receipt["artifacts"] == recipe["artifacts"]
            for name, pin in recipe["artifacts"].items():
                assert sha((root / role / "frontend" / name).read_bytes()) == pin
                assert sha((root / role / "checked-emission/frontend" / name).read_bytes()) == pin
            matcher_bytes(root / role / "frontend/program.o", receipt)
            fault = read(root / role / "promotion-recoveries/result.json")
            assert fault["cases"] == 6 and fault["observations"] == fault["exact_recoveries"] == 12
            proof = read(root / role / "linked-proof.json")
            assert proof["peak_stack_bytes"] <= 5280
            for mode in ("normal", "checked"):
                result = read(root / "public-replays" / (key + "-" + mode) / "result.json")
                assert result["artifacts"] == recipe["artifacts"] and result["exact_measured_artifacts"]
        point = read(root / "source-plan/full96-results.json")
        assert point["complete"] and point["point_passed"] and len(point["runs"]) == 2
        full_calls, outcomes, full_zero = runtime(root / "full81", point["runs"], 81, 96,
                                                  point["plan"]["required_peer_wins"])
        causal = read(root / "source-plan/timing-results.json")
        assert causal["complete"] and len(causal["runs"]) == 2
        causal_calls, _, causal_zero = runtime(root / "runtime", causal["runs"], 21, 120)
        print("Raw runtime records and both full81 point scores verified.", flush=True)
        builds = root / "builds"
        build_plan = read(builds / "plan.json")
        comparison = read(builds / "comparison.json")
        assert len(build_plan["orders"]) == len(comparison["observations"]) == 10
        assert comparison["plan_sha256"] == sha((builds / "plan.json").read_bytes())
        observations = []
        for index, order in enumerate(build_plan["orders"], 1):
            record = {actor: read(builds / f"order-{index}" / actor / "result.json") for actor in order}
            assert record == comparison["observations"][index - 1]
            for actor, result in record.items():
                job = build_plan["actors"][actor]
                assert result["generated_artifacts"] == job["artifact_sha256"]
                assert result["stripped_executable"] == comparison["warmup"][actor]["stripped_executable"]
                if actor.startswith("mech-"):
                    for tag, name in (("encoder", "program.o"), ("metadata", "metadata.json"), ("bytecode", "program.mecb")):
                        assert sha((builds / f"order-{index}" / actor / "frontend" / name).read_bytes()) == job["artifact_sha256"][tag]
            observations.append(record)
        for actor, cuts in comparison["medians"].items():
            for cut, value in cuts.items():
                close(statistics.median(r[actor]["seconds"][cut] for r in observations), value)
        for actor, peers in comparison["paired_ratios"].items():
            for peer, value in peers.items():
                close(statistics.median(r[actor]["seconds"]["source_through_link"] /
                                       r[peer]["seconds"]["source_through_link"] for r in observations), value)
        assert summary["builds"] == read(builds / "summary.json")
        noop = read(root / "current-extrema-disabled/result.json")
        assert noop["same_all_artifacts"] and noop["artifacts"] == noop["enabled_artifacts"]
        print(json.dumps({"passed": True, "source_generator_exact": True,
            "public_native_replays": 4, "full81_orders": outcomes,
            "raw_full81_checked_calls": full_calls, "raw_causal_checked_calls": causal_calls,
            "zero_duration_individual_samples_retained": full_zero + causal_zero,
            "complete_build_orders": 10, "selected_default_changed": False, "next_C_target": 64}))


if __name__ == "__main__":
    main()
