"""Count executed instructions in each matcher's original machine-code blocks."""

from collections import Counter
from hashlib import sha256
from importlib import import_module
import argparse
import json
from pathlib import Path
import re
from shutil import copy2
from struct import pack, pack_into, unpack_from
import subprocess


q = Path(__file__).resolve().parent
layout = import_module("outline-compiled-aarch64-root")
macho = import_module("reachability-census")
text, constant = macho.TEXT, (b"__TEXT", b"__const")
symbol = "_mech_compact_numeric_475_vector"
counter_name = b"_mech_matcher_block_counts\0"
fixtures = q / "fixtures.json"
indices = {v["name"]: i for i, v in enumerate(json.loads(fixtures.read_text()))}


def instructions(path, start, end, allow_data_references=False):
    disassembly = subprocess.check_output(["/usr/bin/otool", "-tvV", path], text=True)
    items = {}
    for line in disassembly.splitlines():
        match = re.fullmatch(r"([0-9a-f]{16})\s+(\S+)\s*(.*)", line)
        if match and start <= (pc := int(match[1], 16)) < end:
            op, operands = match[2], match[3]
            assert op not in {"adr", "bl", "blr", "br", "braa", "brab", "bti"}
            assert allow_data_references or op != "adrp"
            assert allow_data_references or not op.startswith(("ldr", "prfm")) or "[" in operands
            items[pc] = (op, operands)
    assert list(items) == list(range(start, end, 4))
    return items


def replace_atom(path, destination, start, extent, body, new_relocs=(), new_name=b"", moved_relocs=None):
    base, sections, symbols = macho.load(path)
    commands, headers = layout.obj.tables(base)
    assert set(commands) == {0x19, 2, 11} and set(sections) == {text, constant}
    assert not any(unpack_from("<12I", base, commands[11] + 32))
    text_id, address, text_size, text_offset, reloc_at, reloc_count = sections[text]
    const_id, const_address, const_size, const_offset, _, const_relocs = sections[constant]
    assert address == 0 and not const_relocs
    prior_relocs = layout.obj.relocs(base, sections[text])
    contained = {at for at, _ in prior_relocs if start <= at < extent}
    moved_relocs = moved_relocs or {}
    assert set(moved_relocs) == contained
    alignment = 1 << unpack_from("<I", base, headers[text] + 52)[0]
    assert start % alignment == extent % alignment == 0 and alignment >= 4
    padding = (-len(body)) % alignment
    delta = len(body) + padding - (extent - start)
    sys_symbols = commands[2]
    symbols_at, n, strings_at, strings_size = unpack_from("<4I", base, sys_symbols + 8)
    undefined_at, n_undefined = unpack_from("<2I", base, commands[11] + 24)
    assert undefined_at + n_undefined == n == len(symbols) and n < (1 << 24)
    assert not any(label.encode() + b"\0" == new_name for label, *_ in symbols)
    const_end = const_offset + const_size
    assert reloc_at == (const_end + 7) & -8
    assert reloc_at + 8 * reloc_count == symbols_at and symbols_at + 16 * n == strings_at
    assert strings_at + strings_size == len(base)
    old_relocs = [(moved_relocs.get(at, at + (delta if at >= extent else 0)), info)
                  for at, info in prior_relocs]
    fixed = sorted([*new_relocs, *old_relocs], key=lambda pair: -pair[0])
    new_reloc_at = (const_end + delta + 7) & -8
    new_symbol_at = new_reloc_at + 8 * len(fixed)
    header = bytearray(base[:text_offset])
    pack_into("<Q", header, commands[0x19] + 32, const_address + const_size + delta)
    pack_into("<Q", header, commands[0x19] + 48, const_end + delta - text_offset)
    pack_into("<Q", header, headers[text] + 40, text_size + delta)
    pack_into("<2I", header, headers[text] + 56, new_reloc_at, len(fixed))
    pack_into("<Q", header, headers[constant] + 32, const_address + delta)
    pack_into("<I", header, headers[constant] + 48, const_offset + delta)
    symbol_count = n + bool(new_name)
    pack_into("<4I", header, sys_symbols + 8, new_symbol_at, symbol_count,
              new_symbol_at + 16 * symbol_count, strings_size + len(new_name))
    records = bytearray(base[symbols_at:strings_at])
    for index, (_, _, section, pc) in enumerate(symbols):
        if section == const_id or section == text_id and pc >= extent:
            pack_into("<Q", records, index * 16 + 8, pc + delta)
    if new_name:
        pack_into("<I", header, commands[11] + 28, n_undefined + 1)
        records.extend(pack("<IBBHQ", strings_size, 1, 0, 0, 0))
    destination.write_bytes(b"".join((
        header, base[text_offset:text_offset + start], body,
        pack("<I", 0xd503201f) * (padding // 4), base[text_offset + extent:const_end],
        bytes(new_reloc_at - (const_end + delta)), b"".join(pack("<2I", *r) for r in fixed),
        records, base[strings_at:], new_name,
    )))
    result, final_sections, final_symbols = macho.load(destination)
    assert final_sections[text][2] == text_size + delta and len(final_symbols) == symbol_count
    if new_name:
        assert final_symbols[-1] == (new_name[:-1].decode(), 1, 0, 0)
    assert len(layout.obj.relocs(result, final_sections[text])) == len(fixed)
    return text_size + delta


def leaf_code(path, base, text_offset, start, end, literal_start, relocated):
    boundary = literal_start or end
    assert start < boundary <= end
    assembly = instructions(path, start, boundary, allow_data_references=literal_start is not None)
    raw = {pc: unpack_from("<I", base, text_offset + pc)[0] for pc in assembly}
    literals = {}
    for pc, (operation, operands) in assembly.items():
        word = raw[pc]
        if operation == "adrp":
            assert pc in relocated and pc + 4 in relocated
        if word & 0x3b000000 == 0x18000000:
            assert operation == "ldr" and "[" not in operands
            target = layout.branch_target(pc, word, (5, 19))
            assert literal_start is not None and literal_start <= target < end
            literals[pc] = target
    return boundary, assembly, raw, literals


def instrument(path, size, destination, target_symbol=symbol, literal_offset=None):
    base, sections, symbols = macho.load(path)
    _, address, text_size, text_offset, _, _ = sections[text]
    names = macho.text_symbols(sections, symbols)
    start = names[target_symbol]
    final = start + size
    assert address == 0
    extent = min((pc for pc in names.values() if pc > start), default=text_size)
    assert final <= extent
    relocs = [(at, info) for at, info in layout.obj.relocs(base, sections[text]) if start <= at < extent]
    relocated = {at for at, _ in relocs}
    assert all(info >> 28 in (3, 4, 5, 6) for _, info in relocs), relocs  # direct and GOT page pairs
    literal_start = start + literal_offset if literal_offset is not None else None
    end, assembly, raw, literals = leaf_code(path, base, text_offset, start, final, literal_start, relocated)
    leaders = {start}
    edges = {}
    for pc, (operation, _) in assembly.items():
        if field := layout.branch_field(raw[pc]):
            target = layout.branch_target(pc, raw[pc], field)
            assert target in assembly
            edges[pc] = (target, field)
            leaders.add(target)
        if (pc in edges or operation in {"ret", "retab", "udf", "brk"}) and pc + 4 < end:
            leaders.add(pc + 4)
    leaders = sorted(leaders)
    assert len(leaders) <= 1024
    assert assembly[end - 4][0] in {"ret", "retab", "udf", "brk", "b"}

    n = len(symbols)
    counters = {}
    targets = {}
    mapped = {}
    output = []
    new_relocs = []
    for pc in assembly:
        if pc in leaders:
            targets[pc] = start + len(output) * 4
            index = len(counters)
            counters[pc] = index
            # Save both IP registers and restore SP and the registers before
            # the original instruction. This sequence never changes NZCV.
            new_relocs.extend(((targets[pc] + 4, 0x5D000000 | n),
                               (targets[pc] + 8, 0x6C000000 | n)))
            output.extend((0xa9bf47f0, 0x90000010, 0xf9400210, 0xf9400211 | (index << 10),
                           0x91000631, 0xf9000211 | (index << 10), 0xa8c147f0))
        mapped[pc] = start + len(output) * 4
        output.append(raw[pc])
        if pc in edges:
            edges[pc] = (*edges[pc], len(output) - 1)
    for target, field, ix in edges.values():
        output[ix] = layout.branch_opcode(output[ix], start + 4 * ix, targets[target], field)
    if literal_start is not None:
        assert literal_start % 16 == 0
        output.extend([0xd503201f] * ((-len(output)) % 4))
        literal_delta = start + len(output) * 4 - literal_start
        for pc, target in literals.items():
            new_pc = mapped[pc]
            ix = (new_pc - start) // 4
            output[ix] = layout.branch_opcode(output[ix], new_pc, target + literal_delta, (5, 19))
    body = b"".join(pack("<I", word) for word in output)
    body += base[text_offset + end : text_offset + final]
    assert len(body) > extent - start
    replace_atom(path, destination, start, extent, body, new_relocs, counter_name,
                 {pc: mapped[pc] for pc in relocated})
    return assembly, leaders, end


def link_worker(reference, directory, object_file):
    link = json.loads((reference / "runtime.json").read_text())["link"]
    source, = (Path(x) for x in link if x.endswith("/mech-worker.rs"))
    instrumented = directory / "worker.rs"
    copy2(source.with_name("pinned_allocator.rs"), directory / "pinned_allocator.rs")
    code = source.read_text()
    edits = (
        ("use serde::Deserialize;", "use serde::Deserialize;\n#[unsafe(no_mangle)]\n"
         "pub static mut mech_matcher_block_counts: [u64; 1024] = [0; 1024];"),
        ("        let start = Instant::now();", "        unsafe { std::ptr::addr_of_mut!(mech_matcher_block_counts).write([0; 1024]); }\n"
         "        let start = Instant::now();"),
        ("        serde_json::json!({\"case\":case.name,", "        let counts = unsafe { std::ptr::addr_of!(mech_matcher_block_counts).read() };\n"
         "        serde_json::json!({\"block_counts\": &counts[..], \"case\":case.name,"),
    )
    for old, new in edits:
        assert code.count(old) == 1
        code = code.replace(old, new)
    instrumented.write_text(code)
    worker = directory / "mech-worker"
    args = {str(source): str(instrumented), str(reference / "mech-worker"): str(worker),
            f"link-arg={reference / 'program.o'}": f"link-arg={object_file}"}
    assert set(args).issubset(link)
    command = [args.get(value, value) for value in link]
    subprocess.run(command, check=True)
    return worker


def phase_metadata(metadata, target_symbol):
    for phase in metadata["numeric_phases"]:
        for name in (None, "vector_variant", "no_growth_variant", "composed_variant", "grouped_histogram"):
            info = phase if name is None else phase.get(name, {})
            if "_" + info.get("symbol", "") == target_symbol:
                return phase["header"], name, info
    raise ValueError(f"unknown phase: {target_symbol}")


def count(actor, label, cases, target_symbol=symbol, literal_offset=None):
    root = q / f"{label}-native/final-candidate"
    original = root / "program.o"
    metadata = json.loads((root / "metadata.json").read_text())
    _, _, phase = phase_metadata(metadata, target_symbol)
    size = phase["code_bytes"]
    directory = q / "matcher-dynamic-blocks" / actor
    directory.mkdir(parents=True, exist_ok=True)
    output_file = directory / "program.o"
    asm, leaders, body_end = instrument(original, size, output_file, target_symbol, literal_offset)
    worker = link_worker(root, directory, output_file)
    report = {}
    for case in cases:
        result = subprocess.run([worker, root / "metadata.json", fixtures], check=True, capture_output=True,
                                text=True, input=f"C\t{indices[case]}\t0\nNNQ\n")
        assert result.stderr.endswith("CLEANUP private_payload_bytes=0 live_owner_records=0\n")
        rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        assert len(rows) == 2
        assert all(r["exact_reference"] and r["independent_inflation"] and r["input_alias_unchanged"]
                   and r["borrowed_output_alias_unchanged"] for r in rows)
        visits, other = (r["block_counts"] for r in rows)
        assert visits == other and not any(visits[len(leaders):])
        stats = Counter()
        details = []
        for i, (start, end) in enumerate(zip(leaders, [*leaders[1:], body_end], strict=True)):
            times = visits[i]
            if not times:
                continue
            block = [asm[pc] for pc in range(start, end, 4)]
            for op, operands in block:
                stats["instructions"] += times
                stats[op] += times
                if op == "b" or op.startswith(("b.", "cb", "tb")):
                    stats["branches"] += times
                if op.startswith(("ld", "st")):
                    region = "stack" if re.search(r"\[(?:sp|x29|fp)\b", operands) else "payload"
                    stats[region + ("_loads" if op.startswith("ld") else "_stores")] += times
            details.append({"address": hex(start), "visits": times, "instructions": block})
        report[case] = {"calls": visits[0], "counts": dict(stats),
                        "blocks": sorted(details, key=lambda row: -row["visits"])}
        print(actor, case, "calls", visits[0], "instructions", stats["instructions"], "branches", stats["branches"],
              "stack loads", stats["stack_loads"], "stack stores", stats["stack_stores"], flush=True)
    (directory / "counts.json").write_text(json.dumps({"object_sha256": sha256(original.read_bytes()).hexdigest(),
                                                        "symbol": target_symbol,
                                                        "literal_offset": literal_offset,
                                                        "cases": report}, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", action="append", default=[])
    parser.add_argument("--symbol", default=symbol)
    parser.add_argument("--literal-offset", type=lambda value: int(value, 0))
    parser.add_argument("cases", nargs="*")
    arguments = parser.parse_args()
    target = arguments.symbol
    assert re.fullmatch(r"_mech_compact_numeric_[a-z0-9_]+", target)
    cases = arguments.cases or ["prefix-random-1048576", "random-1024", "320-overlap", "prefix-natural-8192"]
    if arguments.candidate:
        for label in arguments.candidate:
            assert re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", label)
            suffix = target.removeprefix("_mech_compact_numeric_").replace("_", "-")
            actor = label if target == symbol else f"{label}-{suffix}"
            count(actor, label, cases, target, arguments.literal_offset)
    else:
        count("cranelift", "cranelift-register-spill-forwarding", cases)
        count("historical", "cranelift-machine-outline-legacy-matcher", cases)
