"""Attribute existing object text and relocations to homologous collector/scorer roles."""

from bisect import bisect_right
from collections import Counter
from hashlib import sha256
from importlib import import_module
import json
from pathlib import Path
from struct import unpack_from
import sys

from control_graph import Q, ROLES, STUDY


sys.path.insert(0, str(Q))
macho = import_module("reachability-census")
CONSTANTS = b"__TEXT", b"__const"
NUMERIC = "_mech_compact_numeric_"
MAIN = {
    "control": {"collector": (475,), "scorer": (3009,)},
    "stored-table-slot": {"collector_children": (940, 2367, 2441, 2652, 2738), "scorer": (3061,)},
    "stored-arithmetic-slot": {"collector": (475,), "scorer": (3085,)},
}


def extents(symbols, end):
    entries = sorted((pc, name) for name, pc in symbols.items())
    return {name: (at, next_at - at) for (at, name), (next_at, _) in zip(entries, entries[1:] + [(end, None)])}


def branch_calls(image, section, symbols, text):
    _, base, _, _, reloc_at, reloc_count = section
    starts = sorted(text.values())
    by_address = {pc: name for name, pc in text.items()}
    result = Counter()
    for index in range(reloc_count):
        pos, flags = unpack_from("<II", image, reloc_at + 8 * index)
        if flags >> 28 != 2 or not (flags >> 27 & 1):
            continue
        caller_address = starts[bisect_right(starts, pos + base) - 1]
        caller = by_address[caller_address]
        target = symbols[flags & 0xffffff][0]
        result[(caller, target)] += 1
    return result


def cleanup_counts(summary):
    fields = ("before_instructions", "after_instructions", "before_blocks", "after_blocks")
    return {key: summary["cleanup"][key] for key in fields if key in summary.get("cleanup", {})}


def census(role):
    folder = STUDY / role
    image, sections, symbols = macho.load(folder / "program.o")
    _, linked_sections, linked_symbols = macho.load(folder / "mech-worker")
    text = macho.text_symbols(sections, symbols)
    linked = macho.text_symbols(linked_sections, linked_symbols)
    section = sections[macho.TEXT]
    index, const_base, const_bytes, *_ = sections[CONSTANTS]
    const_symbols = {name: pc for name, kind, sec, pc in symbols if sec == index and kind & 0xe == 0xe}
    const_symbols = extents(const_symbols, const_base + const_bytes)
    text_extents = extents(text, section[1] + section[2])
    calls = branch_calls(image, section, symbols, text)
    metadata = json.loads((folder / "metadata.json").read_text())
    compile_record = json.loads((folder / "compile.json").read_text())
    assert sha256(image).hexdigest() == compile_record["object_sha256"]
    encoded = {}
    for phase in metadata["numeric_phases"]:
        prefix = f"{NUMERIC}{phase['header']}"
        attr = {key: variant for key, variant in phase.items()
                if isinstance(variant, dict) and "symbol" in variant} | {"general": phase}
        entries = {}
        for key, variant in attr.items():
            symbol = "_" + variant["symbol"]
            if symbol not in text:
                assert not variant.get("selected_by_guard", False), symbol
            entries[key] = {
                "symbol": symbol,
                "compile_cleanup_counts": cleanup_counts(variant),
                "reported_body_bytes": variant["code_bytes"],
                "emitted_extent_bytes": text_extents.get(symbol, (None, 0))[1],
                "linked": symbol in linked,
                "branch_callers": dict(sorted((src, count) for (src, dst), count in calls.items() if dst == symbol)),
                "stack_bytes": variant["stack_bytes"],
            }
        assert {symbol for symbol in text if symbol == prefix or symbol.startswith(prefix + "_")} == {
            entry["symbol"] for entry in entries.values() if entry["emitted_extent_bytes"] > 0
        }
        encoded[phase["header"]] = {
            "input_words": phase["input_words"], "output_words": phase["output_words"],
            "owners_checked": phase["fixed_writable_capacities"], "extra_entry_bounds": phase["phase_entry_bounds"],
            "vector_geometry_guards": phase.get("vector_variant", {}).get("guard_arrays", []),
            "fixed_nested": phase.get("private_fixed_nested"), "variants": entries,
        }
    outlined_bytes = sum(size for name, (_, size) in text_extents.items() if name.startswith(NUMERIC))
    root_bytes = text_extents[macho.ROOT][1]
    return {
        "object_sha256": compile_record["object_sha256"],
        "object_bytes": len(image), "text_bytes": section[2], "const_bytes": const_bytes,
        "composed_cost_code_table_bytes": const_symbols.get("_mech_compact_cost_codes_0", (None, 0))[1],
        "root_body_bytes": metadata["code_bytes"], "root_extent_bytes": root_bytes,
        "root_stack_bytes": metadata["stack_bytes"], "root_cleanup_counts": cleanup_counts(metadata),
        "root_phase_call_storage": metadata["phase_call_storage"],
        "outlined_phase_extents": outlined_bytes, "remaining_text_bytes": section[2] - outlined_bytes - root_bytes,
        "compiled_phases": len(encoded), "phases": encoded,
        "homologous_groups": {name: sum(entry["emitted_extent_bytes"] for header in headers
                                        for entry in encoded[header]["variants"].values())
                             for name, headers in MAIN[role].items()},
        "archived_compile_seconds": compile_record["total_seconds"],
        "archived_native_seconds": next(row["seconds"] for row in compile_record["phases"] if row["phase"] == "native"),
    }


if __name__ == "__main__":
    print(json.dumps({role: census(role) for role in ROLES}, indent=2))
