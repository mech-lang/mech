"""Reconcile existing fixed-reference timings, atom counters, and backing diagnostics."""

import gzip
from hashlib import sha256
import json
from statistics import median
from struct import unpack_from

from control_graph import Q
from text_census import macho


ARCHIVE = Q / "runtime-matchfinder-20261117"
PACKET = Q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
REFERENCE = Q / "native-size-20261117/source-aligned"
SYMBOLS = ("_mech_compact_numeric_475_vector", "_mech_compact_numeric_3009_no_growth_composed")
CASES = ("prefix-natural-4096", "prefix-natural-8192", "prefix-random-4096", "prefix-random-8192", "prefix-random-65536",
         "prefix-random-131072", "prefix-random-262144", "prefix-random-1048576")


def file_hash(path):
    return sha256(path.read_bytes()).hexdigest()


def atoms(folder):
    object_path = folder / "program.o"
    image, sections, symbols = macho.load(object_path)
    _, base, text_size, text_offset, reloc_at, reloc_count = sections[macho.TEXT]
    named = macho.text_symbols(sections, symbols)
    result = {"object_sha256": sha256(image).hexdigest(), "text_bytes": text_size, "atoms": {}}
    for name in SYMBOLS:
        first = named[name]
        last = min([at for at in named.values() if at > first] + [base + text_size])
        body = image[text_offset + first - base:text_offset + last - base]
        relocations = []
        for index in range(reloc_count):
            at, flags = unpack_from("<II", image, reloc_at + index * 8)
            if first <= at + base < last:
                target = symbols[flags & 0xffffff][0] if flags >> 27 & 1 else flags & 0xffffff
                relocations.append((at + base - first, flags >> 24, target))
        result["atoms"][name] = {"extent": len(body), "raw_sha256": sha256(body).hexdigest(),
                                  "relocations": sorted(relocations)}
    return result


def counters(record):
    if record is None:
        return None
    counts = record["counts"]
    total = counts.get("instructions", 0)
    stack_accesses = counts.get("stack_loads", 0) + counts.get("stack_stores", 0)
    assert stack_accesses <= total
    return {"calls": record["calls"], "instructions": total, "stack_accesses": stack_accesses,
            "stack_access_share_of_instructions": stack_accesses / total if total else None}


def archived_costs():
    fixed = atoms(REFERENCE)
    older = atoms(ARCHIVE / "pinned")
    assert fixed["object_sha256"] == "7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b"
    assert fixed["atoms"] == older["atoms"]
    matcher_file = ARCHIVE / "evidence-draft/profiles/matcher-control-counts.json.gz"
    matcher = json.loads(gzip.decompress(matcher_file.read_bytes()))
    scorer_file = PACKET / "evidence/ordered-scorer-addresses/original-counter-visit.json"
    scorer = json.loads(scorer_file.read_text())
    backing_file = ARCHIVE / "payload-time/timing.json"
    backing = json.loads(backing_file.read_text())
    assert scorer["object_sha256"] == fixed["object_sha256"]
    assert matcher["object_sha256"] == backing["object_sha256"] == older["object_sha256"]
    timings = {}
    bindings = {}
    for order in ("forward", "reverse"):
        folder = PACKET / f"evidence/bound-71768-runtime/fully-bound-71768-full-{order}"
        pins = json.loads((folder / "pins.json").read_text())
        worker = pins["actors"]["mech-experiment"]["binary_sha256"]
        assert worker == file_hash(REFERENCE / "mech-worker"), (order, worker)
        summary = folder / "summary.json"
        timings[order] = {row["case"]: row for row in json.loads(summary.read_text())}
        bindings[order] = {"pins_sha256": file_hash(folder / "pins.json"), "summary_sha256": file_hash(summary)}
    cases = {}
    for name in CASES:
        orders = {order: {"x_libdeflate": rows[name]["paired_x_libdeflate"]["mech-experiment"],
                          "median_microseconds": 1e6 * rows[name]["median_seconds"]["mech-experiment"]}
                  for order, rows in timings.items()}
        for row in orders.values():
            row["elapsed_reduction_needed_for_peer_parity"] = max(0.0, 1 - 1 / row["x_libdeflate"])
        profile = backing["cases"].get(name)
        payload = None
        if profile:
            observed = profile["calls"]["profile"]
            median_backing_us = median(sum(row["payload_time_ns"]) for row in observed) / 1000
            payload = {"median_backing_microseconds": median_backing_us,
                       "instrumented_elapsed_microseconds": profile["instrumented_microseconds"],
                       "median_backing_fraction": median_backing_us / profile["instrumented_microseconds"],
                       "zeroed_bytes": observed[0]["zeroed_bytes"],
                       "initialized_bytes_copied": observed[0]["initialized_bytes_copied"]}
        cases[name] = {"fixed_reference_runtime": orders,
                       "matcher_atom_older_trace": counters(matcher["cases"].get(name)),
                       "composed_scorer_atom_exact_trace": counters(scorer["cases"].get(name)),
                       "backing_older_diagnostic": payload}
    return {"exact_reference": fixed, "older_reference": older, "full_timing_receipts": bindings,
            "count_receipts": {"matcher": file_hash(matcher_file), "scorer": file_hash(scorer_file),
                               "backing": file_hash(backing_file)}, "cases": cases}


if __name__ == "__main__":
    print(json.dumps(archived_costs(), indent=2))
