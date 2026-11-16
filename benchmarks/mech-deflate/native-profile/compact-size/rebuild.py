"""Rebuild the measured Apple ARM64 compact Object and verify its identity."""

from hashlib import file_digest
from pathlib import Path
import argparse
import json
import os
import re
import shutil
import subprocess
import sys

import mach_o
from compact_object import replace_text
from error_exits import share_status_exits


ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures"
FLAGS = ["-O=2", "-global-isel=0", "-mcpu=apple-m1", "-relocation-model=pic"]


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def verify_sha(path, expected):
    actual = sha(path)
    if actual != expected:
        raise ValueError(f"{path.name} SHA-256 differs ({actual}, expected {expected})")


def prepare_ir(directory, emitter, program, recipe, llvm):
    prepared = directory / "prepared"
    prepared.mkdir()
    definitions = prepared / "llvm"
    definitions.mkdir()
    names = recipe["coalesced"]
    matcher = recipe["matcher"]
    settings = dict(recipe["emitter_flags"])
    settings.update(
        MECH_COMPACT_DUMP_LLVM_ROOT=str(prepared / "checked.ll"),
        MECH_COMPACT_DUMP_LLVM_DIRECTORY=str(definitions),
        MECH_COMPACT_DUMP_LLVM_SYMBOLS=",".join(sorted([*names, matcher])),
    )
    environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    subprocess.run([emitter, program, prepared], check=True, env=environment | settings)
    obj = prepared / "program.o"
    verify_sha(obj, recipe["prepared_sha256"])
    verify_sha(prepared / "metadata.json", recipe["prepared_metadata_sha256"])
    verify_sha(prepared / "checked.ll", recipe["checked_llvm_sha256"])

    # Let LLVM fold the exact prepared read-only table bytes while the support
    # Object retains their addresses for all references that survive folding.
    image, sections, symbols = mach_o.load(obj)
    sid, address, size, offset, _, relocations = sections[(b"__TEXT", b"__const")]
    assert relocations == 0
    tables = {name[1:]: pc - address for name, kind, section, pc in symbols
              if section == sid and kind == 0xe and name.startswith("_mech_compact_table_")}

    def constant(match):
        name, extent = match[1], int(match[2])
        start = tables[name]
        assert 0 <= start <= size - extent
        value = "".join(f"\\{byte:02X}" for byte in image[offset + start:offset + start + extent])
        return f'@"{name}" = available_externally dso_local constant [{extent} x i8] c"{value}"'

    pattern = r'^@"(mech_compact_table_\d+)" = external dso_local constant \[(\d+) x i8\]$'
    checked = prepared / "checked-with-tables.ll"
    transformed, count = re.subn(pattern, constant, (prepared / "checked.ll").read_text(), flags=re.M)
    assert count > 0
    checked.write_text(transformed)
    linked = directory / "linked.ll"
    subprocess.run([llvm / "llvm-link", "-S", checked, *(definitions / f"{name}.ll" for name in names),
                    "-o", linked], check=True)
    ir = linked.read_text()
    for name in names:
        ir, count = re.subn(rf'(?m)^define (?=[^\n]*@"?{re.escape(name)}"?\()', "define internal ", ir)
        assert count == 1, name
    linked.write_text(ir)
    root = directory / "root.ll"
    subprocess.run([llvm / "opt", "-passes=default<Oz>,verify", "-mcpu=apple-m1", "-S",
                    linked, "-o", root], check=True)
    ir = root.read_text()

    def align(match):
        return f"{match[1]}32{match[3]}" if match[2] in names else match[0]

    ir = re.sub(r'(?m)^(define internal [^\n]*?@([\w.$]+)\([^\n]*?\balign )128(\s*\{)$', align, ir)
    root.write_text(ir)
    matcher_ir = directory / "matcher.ll"
    subprocess.run([llvm / "opt", "-passes=default<Os>,verify", "-mcpu=apple-m1", "-S",
                    definitions / f"{matcher}.ll", "-o", matcher_ir], check=True)
    return obj, root, matcher_ir


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--llvm-bin", type=Path,
                        help="LLVM 22.1.8 bin directory, defaulting to the directory of llc on PATH")
    parser.add_argument("--emitter", type=Path,
                        help="rebuild the prepared Object and LLVM IR with emit_compact_size")
    parser.add_argument("--program", type=Path,
                        help="canonical Program bytecode to use with --emitter")
    args = parser.parse_args()
    if not __debug__:
        parser.error("run without Python's -O flag to keep the Object validation checks")
    if args.program and not args.emitter:
        parser.error("--program requires --emitter")
    llc = shutil.which("llc")
    llvm = args.llvm_bin or (Path(llc).parent if llc else None)
    if llvm is None:
        parser.error("LLVM is missing from PATH. Pass its bin directory with --llvm-bin.")
    llvm = llvm.resolve(strict=True)
    recipe = json.loads((ROOT / "recipe.json").read_text())
    version = subprocess.check_output([llvm / "llc", "--version"], text=True)
    if not re.search(rf"\bversion {re.escape(recipe['llvm_version'])}\b", version):
        parser.error(f"the byte-identical recipe requires LLVM {recipe['llvm_version']}")
    for name, expected in recipe["fixture_sha256"].items():
        verify_sha(FIXTURES / name, expected)

    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    if args.emitter:
        program = (args.program or FIXTURES / "program.mecb").resolve(strict=True)
        prepared, root_ir, matcher_ir = prepare_ir(
            directory, args.emitter.resolve(strict=True), program, recipe, llvm,
        )
    else:
        prepared, root_ir, matcher_ir = (FIXTURES / name for name in ("prepared.o", "root.ll", "matcher.ll"))

    root_assembly = directory / "root.s"
    subprocess.run([llvm / "llc", *FLAGS, "-enable-machine-outliner=always",
                    "-outliner-benefit-threshold=4", "-machine-outliner-reruns=2",
                    *recipe["root_codegen_flags"],
                    "-filetype=asm", root_ir, "-o", root_assembly], check=True)
    shared, sites = share_status_exits(root_assembly.read_text())
    assert sites == recipe["shared_status_sites"], sites
    shared_assembly = directory / "shared.s"
    shared_assembly.write_text(shared)
    compiled = directory / "shared.o"
    subprocess.run([llvm / "llvm-mc", f"-triple={recipe['target']}", "-mcpu=apple-m1",
                    "-filetype=obj", shared_assembly, "-o", compiled], check=True)
    merged = directory / "with-checked-root.o"
    replace_text(prepared, compiled, merged, coalesce=recipe["coalesced"])

    matcher = directory / "matcher.o"
    subprocess.run([llvm / "llc", *FLAGS, "-enable-machine-outliner=never",
                    "-filetype=obj", matcher_ir, "-o", matcher], check=True)
    output = directory / "program.o"
    size = replace_text(merged, matcher, output, symbol=recipe["matcher"])
    assert size == recipe["emitted_text_bytes"], size
    verify_sha(output, recipe["exact_object_sha256"])
    verify_sha(FIXTURES / "metadata.json", recipe["exact_metadata_sha256"])
    shutil.copyfile(FIXTURES / "metadata.json", directory / "metadata.json")
    print(f"{size:,} emitted bytes = {size / recipe['roc_text_bytes']:.3f}× Roc "
          f"and {size / recipe['libdeflate_text_bytes']:.3f}× libdeflate")
    print(f"{sha(output)}  {output}")


if __name__ == "__main__":
    sys.exit(main())
