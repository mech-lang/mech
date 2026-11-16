"""Share ARM64 checked failure returns while preserving status and origin."""

import re


def share_status_exits(assembly):
    """Recognize and replace the original pinned ARM64 checked return sequences."""
    start = assembly.index("\n_mech_compact_closed_checked:")
    end = assembly.index("\t.cfi_endproc", start)
    root = assembly[start:end]
    common = re.search(r"(?m)^(LBB0_\d+):[^\n]*%common\.ret\n\tadd\tsp, sp, #\d+\n", root)
    assert common and root.count(".cfi_restore_state") == 1
    assert re.search(r"\tstp\tx29, x30, \[sp,", root[:common.start()])
    epilogue, ret, _ = root[common.end():].partition("\tret\n")
    assert ret
    # Only the frame restoration follows, so x0 survives and x8/x9 are dead.
    assert re.search(r"\tldp\tx29, x30, \[sp,", epilogue)
    restore = re.compile(r"\tldp\tx(?:19|2[0-9]|30), x(?:19|2[0-9]|30), \[sp(?:, #-?\d+)?\](?:, #\d+)?(?:\s+;.*)?")
    assert all(line.startswith("\t.cfi_") or restore.fullmatch(line) for line in epilogue.splitlines())
    target = common[1]
    move_status = r"\tmov\t(w0, #\d+)[^\n]*\n"
    move_origin = r"\tmov\tw1, #(\d+)[^\n]*\n"
    branch = rf"\tb\t{target}\n"
    tails = re.compile(rf"(?m)^(LBB0_\d+:[^\n]*\n)?(?:(?:{move_status})?{move_origin}|{move_origin}{move_status}){branch}")
    slots = []
    parts = []
    last = 0
    for matched in tails.finditer(root):
        label_text, left_status, left_origin, right_origin, right_status = matched.groups()
        origin = int(left_origin or right_origin)
        operands = left_status or right_status
        assert 0 < origin < 65536
        prior = next(line.strip() for line in reversed(root[:matched.start()].splitlines())
                     if line.strip() and not line.lstrip().startswith(";"))
        parts.append(root[last:matched.start()])
        slot = f"Lmech_status_site_{len(slots)}:\n"
        if label_text and (prior.startswith("b\t") or prior == "ret"):
            slot = label_text + slot
        else:
            parts.extend((label_text or "", f"\tb\tLmech_status_site_{len(slots)}\n"))
        slots.append((slot, origin, operands))
        last = matched.end()
    parts.append(root[last:])
    assert len(slots) >= 2
    root = "".join(parts)
    hint = root.find("\t.loh ")
    finish = len(root) if hint < 0 else hint
    prior = next(line.strip() for line in reversed(root[:finish].splitlines()) if line.strip())
    assert prior.startswith("b\t") or prior == "ret"
    statuses = sorted({operands for _, _, operands in slots if operands})
    handlers = {status: f"Lmech_status_kind_{i + 1}" for i, status in enumerate(statuses)}
    shared = "".join(f"{handler}:\n\tmov\t{status}\n\tb\tLmech_status_from_site\n"
                     for status, handler in handlers.items())
    handlers[None] = "Lmech_status_kind_0"
    shared += f"{handlers[None]}:\n"
    shared += f"""Lmech_status_from_site:
    \tadr\tx8, Lmech_status_sites
    \tsub\tx8, x30, x8
    \tlsr\tx8, x8, #1
    \tadr\tx9, Lmech_status_origins
    \tldrh\tw1, [x9, x8]
    \tb\t{target}
    Lmech_status_sites:
    """
    shared += "".join(name + f"\tbl\t{handlers[operands]}\n" for name, _, operands in slots)
    # The link register points one instruction past its slot.
    shared += "Lmech_status_origins:\n\t.short\t0\n"
    shared += "".join(f"\t.short\t{origin}\n" for _, origin, _ in slots)
    shared += "\t.p2align\t2\n"
    original = assembly[:start]
    return original + root[:finish] + shared + root[finish:] + assembly[end:], len(slots)
