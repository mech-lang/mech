"""Attribute existing checked-root range-proof trace events to source boundaries."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path


ARCHIVES = {
    "old": {
        "compile.log": "e7be5bf0550d44a2da65ea1cb1690802288aaec11782feb3eddcf6aaa010c396",
        "native-exclusive.json": "75ecfc233df183ed4fd089c894ff8bfd0951c9666264b7f53555b8564c1c3344",
    },
    "shared": {
        "compile.log": "c6869232f8b08277eaeb907ccb926fb7d006726b3cf990dbbd4083323d7036f9",
        "native-exclusive.json": "554808a737d756b92125b4930ef4b831e5b8b7069685ae219e7fb7693669fe3b",
    },
}
LABEL = {
    "Control flow graph": "C",
    "Dominator tree": "D",
    "Remove unreachable blocks": "U",
    "Verify Cranelift IR": "V",
    "Remove constant phi-nodes": "P",
}
SEGMENTS = (
    ("checked adds", "CV"),
    ("unsigned ranges and cleanup", "CDCDUVPVV"),
    ("repeated bounds", "CD"),
    ("dominating window facts", "CDCDUVPVV"),
    ("selected read bounds", "CDCDUVPVV"),
    ("checked scalar spans", "CD"),
    ("packed bases", "CD"),
    ("equal join arguments", "CCDUVPVV"),
)
UNCHANGED = ("dominating window facts", "packed bases")
AFTER_REFRESH_AND_PRUNE = ("selected read bounds", "checked scalar spans")


def pinned(folder, filename, digest):
    data = gzip.decompress((folder / f"{filename}.gz").read_bytes())
    assert hashlib.sha256(data).hexdigest() == digest, filename
    return data


def report(archive, variant):
    folder = archive / variant
    log, trace = (pinned(folder, key, sha) for key, sha in ARCHIVES[variant].items())
    assert b"compact root proved carries: 26\n" in log
    assert b"compact root range proofs: 190 unsigned, 0 repeated, 6 windows, 145 reads, 0 spans, 0 packed\n" in log
    assert b"compact root flow proofs: 5 equal arguments" in log
    coordinator, = (thread["events"] for thread in json.loads(trace) if thread["thread"] == "coordinator")
    root, = (i for i, row in enumerate(coordinator)
             if row["category"] == "cfg_cleanup" and row["detail"] == "checked root")
    direct = [(i, row) for i, row in enumerate(coordinator) if row["parent"] == root]
    labels = "".join(LABEL.get(row["detail"], "?") for _, row in direct)
    signature = "".join(pattern for _, pattern in SEGMENTS)
    assert labels.count(signature) == 1, labels
    start = labels.index(signature)
    result = {}
    for segment, pattern in SEGMENTS:
        if segment in UNCHANGED + AFTER_REFRESH_AND_PRUNE:
            pair = direct[start:start + 2]
            assert [row["detail"] for _, row in pair] == ["Control flow graph", "Dominator tree"]
            result[segment] = {
                "first_graph_pair_events": [index for index, _ in pair],
                "graph_pair_elapsed_ns": sum(row["end_ns"] - row["start_ns"] for _, row in pair),
            }
        start += len(pattern)
    unchanged = sum(result[name]["graph_pair_elapsed_ns"] for name in UNCHANGED)
    additional = sum(result[name]["graph_pair_elapsed_ns"] for name in AFTER_REFRESH_AND_PRUNE)
    return {"checked_root_event": root, "range_entry_pairs": result,
            "unchanged_boundaries_ns": unchanged, "after_pruning_boundaries_ns": additional,
            "total_measured_work_ns": unchanged + additional}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path, help="published native-exclusive-work directory")
    args = parser.parse_args()
    print(json.dumps({variant: report(args.archive, variant) for variant in ARCHIVES}, indent=2))


if __name__ == "__main__":
    main()
