def leaf_code(path, base, text_offset, start, end, literal_start, relocated):
    boundary = literal_start or end
    assert start < boundary <= end
    assembly = instructions(path, start, boundary, allow_data_references=True)
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
