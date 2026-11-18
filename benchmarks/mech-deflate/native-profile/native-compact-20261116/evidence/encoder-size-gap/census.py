"""Attribute measured AArch64 encoder sizes and replay the C outlining comparison."""

import argparse
from collections import Counter
from hashlib import file_digest
import json
from pathlib import Path
import re
import subprocess
from runpy import run_path


HERE = Path(__file__).resolve().parent
MACHO = run_path(str(HERE.parent / "host-opt-levels/scripts-as-run/reachability-census.py"))
ROOT = "_mech_compact_closed_checked"
CLANG = "/usr/bin/clang"


def sha(path):
    with open(path, 'rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def atoms(path):
    data, sections, names = MACHO["load"](path)
    _, base, size, _, _, _ = sections[MACHO["TEXT"]]
    defined = sorted((pc, name) for name, pc in MACHO["text_symbols"](sections, names).items()
                     if name.startswith('_'))
    assert defined[0][0] == base
    assert len({pc for pc, _ in defined}) == len(defined)
    extents = {}
    for i, (start, name) in enumerate(defined):
        end = defined[i+1][0] if i+1 < len(defined) else base + size
        extents[name] = {'start': start, 'bytes': end-start}
    assert sum(a['bytes'] for a in extents.values()) == size
    return {'sha256': sha(path), 'file_bytes': len(data), 'text_bytes': size,
            'symbols': extents, 'sections': {','.join(k.decode() for k in s): d[2]
                                             for s, d in sections.items()}}


def root_categories(path, begin, size):
    dump = subprocess.check_output(['/usr/bin/otool', '-tvV', str(path)], text=True)
    count = Counter()
    for line in dump.splitlines():
        match = re.fullmatch(r'([0-9a-f]{16})\s+(\S+)(?:\s+(.*))?', line)
        if not match:
            continue
        addr, opcode, arg = match.groups()
        if not begin <= int(addr, 16) < begin + size:
            continue
        if opcode in ('bl', 'blr'):
            kind = 'call'
        elif opcode in ('b', 'br', 'ret', 'retab'):
            kind = 'unconditional control'
        elif opcode.startswith(('b.', 'cbz', 'cbnz', 'tbz', 'tbnz')):
            kind = 'conditional control'
        elif opcode.startswith(('ld', 'st')):
            kind = 'stack memory' if re.search(r'\[(?:sp|x29)(?:[,\]])', arg or '') else 'other memory'
        else:
            kind = 'other or data'
        count[kind] += 4
    assert sum(count.values()) == size, (count, size)
    return dict(count.most_common())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mech", type=Path, help="directory containing program.o and metadata.json")
    parser.add_argument("libdeflate", type=Path, help="source tree of the pinned libdeflate")
    parser.add_argument("output", type=Path, help="new directory for peer objects and census")
    parser.add_argument("--historical", type=Path, help="optional historical program.o and metadata.json")
    args = parser.parse_args()
    args.output.mkdir()
    report = {"toolchain": subprocess.check_output([CLANG, "--version"], text=True).strip()}
    for label, folder in [("mech", args.mech), ("historical-mech", args.historical)]:
        if folder is None:
            continue
        obj = folder / "program.o"
        metadata = json.loads((folder / "metadata.json").read_text())
        measurement = atoms(obj)
        root = measurement["symbols"][ROOT]["start"]
        measurement["checked_root_body"] = metadata["code_bytes"]
        measurement["checked_root_instruction_classes"] = root_categories(
            obj, root, metadata["code_bytes"]
        )
        measurement["checked_root_stack_frame"] = metadata["stack_bytes"]
        report[label] = measurement
    source = args.libdeflate / "lib/deflate_compress.c"
    report["libdeflate_source_sha256"] = sha(source)
    for label, options in [("peer-rebuilt", []), ("peer-no-outlining", ["-mno-outline"])]:
        obj = args.output / f"{label}.o"
        command = [CLANG, *options, "-arch", "arm64", "-std=gnu11", "-O2", "-DNDEBUG",
                   "-Wall", "-Werror", "-I", str(args.libdeflate), "-c", str(source), "-o", str(obj)]
        subprocess.run(command, check=True)
        report[label] = atoms(obj) | {"command": command}
    (args.output / "census.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
