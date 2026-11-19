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
    leaders.add(winner_site)
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
        if pc == winner_site:
            at = start + len(output) * 4
            new_relocs.extend(((at + 4, 0x5D000000 | n), (at + 8, 0x6C000000 | n)))
            # The same IP-register save/restore as block probes. CINC and the
            # other instructions preserve NZCV for the original CSEL.
            index = 1023
            output.extend((0xa9bf47f0, 0x90000010, 0xf9400210, 0xf9400211 | (index << 10),
                           conditional_increment, 0xf9000211 | (index << 10), 0xa8c147f0))
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
