"""Align untimed scorer work, using actual scalar and SIMD block visits.

Arguments are staged files, so this verification runs without a compiler or M1.
The native counter data comes from the pre-layout body of the selected recipe.
The generation receipts must differ only by that scorer's block-layout profile.
"""

import argparse
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import re


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def annotate(counts):
    blocks = {}
    for case in counts["cases"].values():
        for block in case["blocks"]:
            at = block["address"]
            if at in blocks:
                assert blocks[at] == block["instructions"]
            blocks[at] = block["instructions"]
    annotations = {}
    for at, instructions in blocks.items():
        # Full 8-byte DP-node stores occur on the generic and forwarded routes.
        stores = sum(op == "str" and bool(re.fullmatch(
            r"x\d+, \[x\d+, x\d+, lsl #3\]", args)) for op, args in instructions)
        literal_only = len(instructions) == 1 and instructions[0][0] == "cbz" and instructions[0][1].startswith("x4,")
        records = sum(op == "ldrb" for op, _ in instructions)
        # Each UZP1 projects four 32-bit costs from four 8-byte DP nodes.
        # The eight-entry loop has two projections; the four-entry loops one.
        vector = 4 * sum(op == "uzp1.4s" for op, _ in instructions)
        # A scalar candidate loads its length price and next-node cost, adds
        # offset+length+tail, then conditionally updates the chosen packed item.
        scalar = int(
            sum(op in ("ldr", "ldur") for op, _ in instructions) == 2
            and any(op == "orr" for op, _ in instructions)
            and any(op == "csel" for op, _ in instructions)
            and sum(op == "add" and args.startswith("w") for op, args in instructions) >= 2
        )
        values = {"dp_positions": stores,
                  "dp_literal_only_positions": int(literal_only),
                  "dp_match_records": records,
                  "candidate_lengths": vector + scalar}
        if any(values.values()):
            annotations[at] = {"weights": values, "instructions": instructions}
    assert sum(x["weights"]["dp_positions"] > 0 for x in annotations.values()) == 2
    assert sum(x["weights"]["dp_literal_only_positions"] for x in annotations.values()) == 1
    assert sum(x["weights"]["dp_match_records"] for x in annotations.values()) == 1
    assert sum(x["weights"]["candidate_lengths"] > 0 for x in annotations.values()) == 5
    return annotations


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--c-summary", type=Path, required=True)
    parser.add_argument("--mech-counts", type=Path, required=True)
    parser.add_argument("--raw-receipt", type=Path, required=True)
    parser.add_argument("--profiled-receipt", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    c, m = read(args.c_summary), read(args.mech_counts)
    raw, profiled = read(args.raw_receipt), read(args.profiled_receipt)
    assert m["object_sha256"] == raw["object_sha256"]
    assert raw["bytecode_sha256"] == profiled["bytecode_sha256"]
    differences = {key: [raw["flags"].get(key), profiled["flags"].get(key)]
                   for key in raw["flags"].keys() | profiled["flags"].keys()
                   if raw["flags"].get(key) != profiled["flags"].get(key)}
    assert set(differences) == {"MECH_COMPACT_PHASE_BLOCK_LAYOUT"}
    before, after = differences["MECH_COMPACT_PHASE_BLOCK_LAYOUT"]
    assert after.startswith(before + ";mech_compact_numeric_3009_no_growth_composed@")
    assert ";" not in after[len(before) + 1:]
    annotations = annotate(m)
    rows = {}
    assert set(m["cases"]) == set(c["cases"])
    for name, case in m["cases"].items():
        measured = Counter(passes=case["calls"])
        for block in case["blocks"]:
            for key, weight in annotations.get(block["address"], {}).get("weights", {}).items():
                measured[key] += block["visits"] * weight
        peer = c["cases"][name]
        expected = {"passes": len(peer["passes"])}
        expected.update({key: peer["counters"][key] for key in measured if key != "passes"})
        assert dict(measured) == expected, (name, measured, expected)
        rows[name] = {"mech": dict(measured), "libdeflate": expected, "all_equal": True,
                      "mech_scorer_instructions": case["counts"]["instructions"],
                      "mech_scorer_stack_loads": case["counts"].get("stack_loads", 0)}
    result = {"cases": rows, "all_equal": True, "block_annotations": annotations,
        "input_sha256": {key: sha(value) for key, value in vars(args).items() if key != "output"},
        "mech_raw_object_sha256": raw["object_sha256"],
        "mech_profiled_object_sha256": profiled["object_sha256"],
        "bytecode_sha256": raw["bytecode_sha256"],
        "source_commit": raw["source_base"], "layout_only_difference": differences,
        "scope": "Exact measured minimum-cost scorer work, before its block scheduling: calls/passes, node writes, no-match cache headers, consumed match records and candidate lengths including SIMD lanes. Both selected and carried-cost bodies are checked separately. This does not establish equal matcher, Huffman, setup or output work, nor equal per-pass purpose; it is not a latency measurement."}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"all_equal": True, "cases": len(rows), "source_commit": raw["source_base"]}))


if __name__ == "__main__":
    main()
