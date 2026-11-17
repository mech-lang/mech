"""Derive a conservative scalar transport census from the exact retained source CFG dump."""

import argparse
from collections import Counter, deque
import gzip
from hashlib import file_digest
import json
from pathlib import Path
import re


Q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
DUMP = Q / "native-compile-dead-exits/ir"
FIXED = Q / "native-size-20261117/source-aligned"
PINS = {
    "program.o": "7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b",
    "program.mecb": "c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3",
    "metadata.json": "cc65be367c7e31a02539d520cb9698bb715bb1c4575c72ffd328934f982db72e",
    "dump/source-cfg.json": "d24007e2a49c55f28dbf32304fecc81a864e5723284cd12c02f48fd14c889ad4",
    "mech-worker": "674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9",
}
VALUE = r"NativeMatchValue\((\d+)\)"
PACK_WORD = re.compile(rf"Scalar\({VALUE}\)|Field\({VALUE}, (\d+)\)|Constant\((\d+)\)")
SIMPLE = {"Scalar(Complete)", "Scalar(Reject)", "Scalar(IntegrityFailure)"}


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def pinned_json(path, expected):
    open_file = gzip.open if path.suffix == ".gz" else open
    with open_file(path, "rb") as stream:
        assert file_digest(stream, "sha256").hexdigest() == expected, path
    with open_file(path, "rb") as stream:
        return json.load(stream)


def tail_iterations(text):
    roots = list(re.finditer(r"(?m)^    (Tail|Collection|Iteration)\(", text))
    result = {}
    for index, root in enumerate(roots):
        if root[1] != "Tail":
            continue
        end = roots[index + 1].start() if index + 1 < len(roots) else len(text)
        block = text[root.end():end]
        record = {}
        for key in ("seed", "next", "carrier"):
            record[key] = int(re.search(rf"\b{key}: NativeMatchValue\(\s*(\d+)", block)[1])
        for key in ("initialize", "header", "latch", "exit"):
            record[key] = int(re.search(rf"\b{key}: (\d+),", block)[1])
        result[index] = record
    return result


def scalar_ordinals(ty, base=0):
    kind = ty["kind"]
    if "product" in kind:
        return tuple(ordinal for field in kind["product"]
                     for ordinal in scalar_ordinals(field["type"], base + field["offset"]))
    if "scalar_bytes" in kind and ty["words"] == 1:
        return (base,)
    return ()


def transport(row, values, tails):
    recipe = row["recipe"]
    match = re.fullmatch(rf"Pack \{{ output: {VALUE}, words: \[(.*)\] \}}", recipe)
    if match:
        output = int(match[1])
        sources = []
        remaining = match[2]
        while remaining:
            word = PACK_WORD.match(remaining)
            assert word is not None, remaining
            scalar, source, ordinal, constant = word.groups()
            if scalar is not None:
                sources.append(int(scalar))
            elif source is not None:
                value = values[int(source)]
                assert int(ordinal) < value["words"]
                sources.append(value["base"] + int(ordinal))
            else:
                assert constant is not None
                sources.append(None)
            remaining = remaining[word.end():]
            if remaining:
                assert remaining.startswith(", ")
                remaining = remaining[2:]
        operation = "pack"
    else:
        match = re.fullmatch(rf"Copy \{{ source: {VALUE}, output: {VALUE} \}}", recipe)
        if match:
            source, output = map(int, match.groups())
            operation = "copy"
        else:
            match = re.fullmatch(r"(Initialize|Latch) \{ iteration: (\d+) \}", row["action"])
            if not match or int(match[2]) not in tails:
                return None
            tail = tails[int(match[2])]
            seed = match[1] == "Initialize"
            assert row["site"] == tail["initialize" if seed else "latch"]
            source, output = tail["seed" if seed else "next"], tail["carrier"]
            operation = "tail-seed" if seed else "tail-latch"
        value = values[source]
        sources = list(range(value["base"], value["base"] + value["words"]))
    assert row["defs"] == [output]
    destination = values[output]
    assert len(sources) == destination["words"]
    return operation, output, sources


def value_words(values, index):
    value = values[index]
    return range(value["base"], value["base"] + value["words"])


def route(row, values):
    term = row["terminator"]
    if match := re.fullmatch(r"Scalar\(Jump\((\d+)\)\)", term):
        return (int(match[1]),), ()
    if match := re.fullmatch(rf"Scalar\(Branch \{{ condition: {VALUE}, yes: (\d+), no: (\d+) \}}\)", term):
        condition, yes, no = map(int, match.groups())
        return (yes, no), tuple(value_words(values, condition))
    if match := re.fullmatch(rf"Scalar\(LiteralBranch \{{ left: {VALUE}, right: {VALUE}, yes: (\d+), no: (\d+) \}}\)", term):
        left, right, yes, no = map(int, match.groups())
        return (yes, no), (*value_words(values, left), *value_words(values, right))
    if match := re.fullmatch(r"TailHeader \{ iteration: \d+, body: (\d+) \}", term):
        return (int(match[1]),), ()
    if match := re.fullmatch(r"Header \{ iteration: \d+, body: (\d+), exit: (\d+) \}", term):
        return tuple(map(int, match.groups())), ()
    assert term in SIMPLE, term
    if term == "Scalar(Complete)":
        return (), tuple(word for value in row["uses"] for word in value_words(values, value))
    return (), ()


def exact_read(row, values):
    match = re.fullmatch(rf"Project \{{ source: {VALUE}, offset: (\d+), guards: (\[.*\]), output: {VALUE} \}}", row["recipe"])
    if match:
        source, offset, guards, output = match.groups()
        width = values[int(output)]["words"]
    else:
        match = re.fullmatch(rf"Observe \{{ source: {VALUE}, offset: (\d+), ordinal: (?:None|Some\(\d+\)), guards: (\[.*\]), output: {VALUE} \}}", row["recipe"])
        if not match:
            return None
        source, offset, guards, output = match.groups()
        width = 1
    value = values[int(source)]
    count = value["words"]
    offsets = list(range(int(offset), int(offset) + width))
    offsets.extend(int(i) for i in re.findall(r"\((\d+), \d+\)", guards))
    assert all(ordinal < count for ordinal in offsets)
    return [value["base"] + ordinal for ordinal in offsets]


def boundaries(metadata, sites, tails):
    result = {}

    def add(name, members):
        for member in members:
            result.setdefault(member, set()).add(name)

    for phase in metadata["numeric_phases"]:
        add("phase", (phase["header"], phase["exit"]))
        add("handoff", (row["site"] for row in phase.get("private_fixed_nested", {}).get("cold_exports", [])))
    for pack in metadata["word_pack_sites"]:
        add("packed words", range(pack["first"], pack["next"] + 1))
    for chain in metadata["private_indexed_chains"]:
        add("private chain", [*chain["sites"], chain["exit"]])
    for header in metadata["widening_reductions"]:
        tail, = [t for t in tails.values() if t["header"] == header]
        add("widened reduction", (header, tail["exit"]))
    for site, row in enumerate(sites):
        if row["terminator"].startswith("Header ") or row["action"].startswith(("Collection", "PrepareU64Range")):
            add("collection", (site,))
    assert not any(key in metadata for key in ("overwrite_reuse", "overwritten_results"))
    return result


def inventory(graph, metadata, scope, *, failure_barrier=True, opaque_routes=True):
    rows, values = graph["sites"], graph["values"]
    tails = tail_iterations(graph["iterations"])
    protected = boundaries(metadata, rows, tails) if opaque_routes else {}
    if failure_barrier:
        for row in rows:
            if row["terminator"] in {"Scalar(Reject)", "Scalar(IntegrityFailure)"}:
                protected.setdefault(row["site"], set()).add("checked failure")
    candidates = {}
    coarse = Counter()
    declines = Counter()
    shape_counts = Counter()
    for index, row in enumerate(rows):
        assert row["site"] == index
        if row["block"] is None or (transfer := transport(row, values, tails)) is None:
            continue
        kind, output, sources = transfer
        coarse[(kind, "reachable_sites")] += 1
        coarse[(kind, "all_words")] += len(sources)
        value = values[output]
        is_product = "product" in value["type"]["kind"]
        ordinals = scalar_ordinals(value["type"])
        whole_scalar = len(ordinals) == len(sources) and not value["owners"]
        if is_product and whole_scalar:
            shape = "all-scalar product"
        elif is_product and ordinals:
            shape = "mixed product with plain scalar siblings"
        elif whole_scalar:
            shape = "single scalar"
        else:
            shape = "protected shape"
        shape_counts[(kind, shape, "sites")] += 1
        shape_counts[(kind, shape, "scalar_words")] += len(ordinals)
        allowed = {
            "product": {"all-scalar product"},
            "pure": {"all-scalar product", "single scalar"},
            "mixed": {"all-scalar product", "mixed product with plain scalar siblings"},
        }[scope]
        if shape not in allowed:
            declines[(kind, shape)] += 1
            continue
        if index in protected:
            declines[(kind, "synthetic/checked boundary")] += 1
            continue
        if scope != "mixed" and (row["transfer"] or row["retire_after"]):
            declines[(kind, "owner traffic at a scalar site")] += 1
            continue
        candidates[index] = (*transfer, ordinals)
    keys = {values[output]["base"] + ordinal for _, output, _, ordinals in candidates.values() for ordinal in ordinals}
    bits = {slot: 1 << index for index, slot in enumerate(sorted(keys))}
    all_live = (1 << len(bits)) - 1

    def mask(slots):
        result = 0
        for slot in slots:
            result |= bits.get(slot, 0)
        return result

    pred = [[] for _ in rows]
    successors, required, post, moves, killed = [None] * len(rows), [], [], [], []
    for index, row in enumerate(rows):
        edges, terminal_reads = route(row, values)
        successors[index] = edges
        for target in edges:
            pred[target].append(index)
        post.append(mask(terminal_reads))
        killed.append(0)
        moves.append(())
        if index in protected:
            required.append(all_live)
        elif index in candidates:
            _, output, sources, ordinals = candidates[index]
            base = values[output]["base"]
            killed[index] = mask(base + ordinal for ordinal in ordinals)
            moves[index] = tuple((bits[base + ordinal], bits.get(sources[ordinal], 0)) for ordinal in ordinals)
            required.append(mask(src for ordinal, src in enumerate(sources) if ordinal not in ordinals))
        elif (reads := exact_read(row, values)) is not None:
            required.append(mask(reads))
        else:
            required.append(mask(word for use in row["uses"] for word in value_words(values, use)))
    live_before = [0] * len(rows)
    pending = deque(range(len(rows) - 1, -1, -1))
    queued = set(pending)
    visits = 0
    while pending:
        visits += 1
        if visits > 500_000:
            return {"analysis": "budget exhausted"}
        site = pending.popleft()
        queued.remove(site)
        after = post[site]
        for next_site in successors[site]:
            after |= live_before[next_site]
        new = required[site] | (after & ~killed[site])
        for dst, src in moves[site]:
            if after & dst:
                new |= src
        assert not (live_before[site] & ~new), site
        if new != live_before[site]:
            live_before[site] = new
            for previous in pred[site]:
                if previous not in queued:
                    pending.append(previous)
                    queued.add(previous)
    result = Counter()
    omissions = []
    for site, (kind, output, sources, ordinals) in candidates.items():
        result[(kind, "candidate_sites")] += 1
        result[(kind, "candidate_words")] += len(ordinals)
        after = post[site]
        for successor in successors[site]:
            after |= live_before[successor]
        lanes = [ordinal for ordinal, (dst, _) in zip(ordinals, moves[site], strict=True) if not after & dst]
        if lanes:
            result[(kind, "sites_with_omissions")] += 1
            result[(kind, "omitted_reads")] += sum(sources[index] is not None for index in lanes)
            result[(kind, "omitted_constants")] += sum(sources[index] is None for index in lanes)
            row = rows[site]
            omissions.append({"site": site, "origin": row["origin"], "kind": kind,
                              "output": output, "omit": lanes,
                              "owner_transfer": bool(row["transfer"]),
                              "owner_retirement": bool(row["retire_after"])})
    counters = coarse | result
    families = sorted({family for family, category in counters})
    return {"analysis": "fixed point", "fixed_point_site_visits": visits,
            "tracked_scalar_words": len(bits), "barrier_sites": len(protected),
            "counts": {family: {category: value for (actual, category), value in counters.items() if actual == family}
                       for family in families},
            "declines": {family: {why: count for (actual, why), count in declines.items() if actual == family}
                         for family in families},
            "shapes": [{"operation": kind, "shape": shape, "metric": metric, "count": count}
                       for (kind, shape, metric), count in sorted(shape_counts.items())], "omissions": omissions}


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("--source-cfg", type=Path, default=DUMP / "dump/source-cfg.json")
    args.add_argument("--metadata", type=Path, default=DUMP / "metadata.json")
    args.add_argument("--check-original-artifacts", action="store_true")
    flags = args.parse_args()
    graph = pinned_json(flags.source_cfg, PINS["dump/source-cfg.json"])
    metadata = pinned_json(flags.metadata, PINS["metadata.json"])
    if flags.check_original_artifacts:
        for folder, names in ((DUMP, ("program.o", "program.mecb")),
                              (FIXED, ("program.o", "metadata.json", "mech-worker"))):
            for name in names:
                assert digest(folder / name) == PINS[name], name
    assert [row["origin"] for row in graph["sites"]] == metadata["origins"]
    reachable = {0}
    work = [0]
    for index in work:
        for successor in route(graph["sites"][index], graph["values"])[0]:
            if successor not in reachable:
                reachable.add(successor)
                work.append(successor)
    assert reachable == {row["site"] for row in graph["sites"] if row["block"] is not None}
    assert len(reachable) == metadata["reachable_blocks"]
    print(json.dumps({"pinned_archives": PINS,
                      "reconstructed_sites": len(graph["sites"]),
                      "reachable_sites": len(reachable),
                      "plan_values": len(graph["values"]),
                      "strict_pure_products": inventory(graph, metadata, "product"),
                      "strict_with_single_word_copies": inventory(graph, metadata, "pure"),
                      "protected_siblings_sensitivity": inventory(graph, metadata, "mixed"),
                      "source_only_product_sensitivity": inventory(graph, metadata, "product", failure_barrier=False, opaque_routes=False),
                      "source_only_protected_siblings_sensitivity": inventory(graph, metadata, "mixed", failure_barrier=False, opaque_routes=False)}, indent=2))


if __name__ == "__main__":
    main()
