"""Replay archive identities, validation and semantic alignment without native tools."""

import ast
from collections import Counter
import gzip
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile


HERE = Path(__file__).resolve().parent


def read(path):
    return json.loads(Path(path).read_text())


def sha(payload):
    return sha256(payload).hexdigest()


def main():
    summary = read(HERE / "summary.json")
    archives = read(HERE / "archive-files.json")
    assert summary["archives"] == archives
    assert summary["selected"]["libdeflate_wins"] == 63
    assert summary["next_C_milestone"] == 64 and summary["ultimate_C_target"] == 81
    assert not summary["carry_candidate"]["selected"]
    assert not summary["carry_candidate"]["full81_qualified"]
    with tempfile.TemporaryDirectory(prefix="mech-current-work-") as temp:
        out = Path(temp)
        for name, pin in archives.items():
            path = HERE / name
            assert sha(path.read_bytes()) == pin["sha256"], name
            with tarfile.open(path) as tar:
                assert len(tar.getmembers()) == len(pin["members"])
                assert {m.name for m in tar.getmembers()} == set(pin["members"])
                for member in tar.getmembers():
                    assert member.isfile() and not member.name.startswith("/")
                    assert ".." not in Path(member.name).parts
                    assert sha(tar.extractfile(member).read()) == pin["members"][member.name]
                tar.extractall(out, filter="data")
        owners = read(out / "owners/summary.json")
        assert owners["complete"]
        classifier = out / "owner-classifier.py"
        scope = {"re": re, "Counter": Counter, "payload": lambda p: Path(p).read_bytes()}
        nodes = [n for n in ast.parse(classifier.read_text()).body
                 if isinstance(n, ast.FunctionDef) and n.name == "parse"]
        assert len(nodes) == 1
        exec(compile(ast.Module(body=nodes, type_ignores=[]), str(classifier), "exec"), scope)
        captures = 0
        for case, actors in owners["profiles"].items():
            for name, row in actors.items():
                folder = out / "owners" / case / name
                assert sha((folder / "sample.txt").read_bytes()) == row["sample_sha256"]
                assert row == read(folder / "result.json")
                kind = owners["plan"]["actors"][name]["kind"]
                parsed = scope["parse"](folder / "sample.txt", kind)
                assert json.loads(json.dumps(parsed)) == row["stats"]
                assert len(row["verified_calls"]) == 2
                for checked in row["verified_calls"]:
                    assert all(checked[key] for key in (
                        "independent_inflation", "input_alias_unchanged", "borrowed_output_alias_unchanged"))
                captures += 1
        assert captures == 45
        c = read(out / "libdeflate/summary.json")
        assert c["complete"] and len(c["cases"]) == 9
        assert c["script_sha256"] == sha((HERE / "trace-libdeflate.py").read_bytes())
        vectors = 0
        for case, row in c["cases"].items():
            path = out / "libdeflate" / (case + ".json")
            assert sha(path.read_bytes()) == row["trace_sha256"]
            assert row["independent_inflation"] and row["matches_pinned_worker"]
            trace = read(path)
            assert trace["counters"] == row["counters"] and trace["passes"] == row["passes"]
            assert row["counters"]["dp_positions"] == sum(p["length"] for p in row["passes"])
            assert all(p["positions"] == p["length"] == p["chosen_literals"] + p["chosen_match_bytes"]
                       for p in row["passes"])
            vectors += len(trace["frequency_vectors"])
        assert vectors == summary["C_frequency_vectors"]
        for role in ("narrow-packet-eight-probe", "parallel-low-packet-probe"):
            root = out / role
            raw = read(root / "raw-receipt.json")
            profiled = read(root / "profiled-receipt.json")
            assert sha((root / "frontend/program.o").read_bytes()) == raw["object_sha256"]
            assert sha((root / "frontend/metadata.json").read_bytes()) == raw["metadata_sha256"]
            assert sha((root / "frontend/program.mecb").read_bytes()) == raw["bytecode_sha256"]
            matcher = "_mech_compact_numeric_475_vector"
            assert raw["layout"]["symbols"][matcher] == profiled["layout"]["symbols"][matcher]
            result = out / (role + "-alignment-replay.json")
            subprocess.run([sys.executable, str(HERE / "align-dp-work.py"),
                "--c-summary", str(out / "libdeflate/summary.json"),
                "--mech-counts", str(root / "counters/counts.json"),
                "--raw-receipt", str(root / "raw-receipt.json"),
                "--profiled-receipt", str(root / "profiled-receipt.json"),
                "--output", str(result)], check=True)
            assert read(result) == read(root / "alignment.json")
        carry = read(out / "parallel-low-selected63-discovery-parallel-low-packet-probe-profiled-results.json")
        assert carry["complete"] and len(carry["runs"]) == 2
        calls = 0
        for run in carry["runs"]:
            folder = out / "runtime" / Path(run["run"]).name
            assert sha((folder / "summary.json").read_bytes()) == run["summary_sha256"]
            assert len(run["results"]) == 21
            for case in run["results"]:
                observations = json.loads(gzip.decompress(
                    (folder / case["case"] / "observations.json.gz").read_bytes()))
                assert len(observations) == 24
                for block in observations:
                    assert set(block["observations"]) == {"mech-control", "mech-experiment", "roc", "libdeflate"}
                    for rows in block["observations"].values():
                        for row in rows:
                            assert all(row[key] for key in ("independent_inflation", "input_alias_unchanged", "borrowed_output_alias_unchanged"))
                            calls += 1
                row = summary["carry_candidate"]["runtime"][run["order"]][case["case"]]
                assert row["all_observed_control_C_wins_retained"]
                assert row["all_observed_control_Roc_wins_retained"]
        assert len(read(out / "narrow-packet-eight-probe/matcher-counters/counts.json")["cases"]) == 9
        print(json.dumps({"passed": True, "archives": len(archives), "current_owner_captures": captures,
                          "C_frequency_vectors": vectors, "aligned_DP_cases_per_Mech_body": 9,
                          "carry_raw_checked_calls": calls, "selected_C_wins": 63, "next_target": 64}))


if __name__ == "__main__":
    main()
