"""Restore the benchmark's function alignments after shortening the checked root."""

from hashlib import file_digest
from importlib import import_module
import json
from pathlib import Path
from struct import pack
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
trace = import_module("count-matcher-instructions")
macho = trace.macho
old = q / "stale-cache-accounting/pinned-smaller"
new = q / "branch-on-zero-native/final-candidate"
label = "branch-on-zero-aligned"
output = q / f"{label}-native/final-candidate"
output.mkdir(parents=True)


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def symbols(file):
    _, sections, symbol_table = macho.load(file)
    return macho.text_symbols(sections, symbol_table)


old_symbols = symbols(old / "program.o")
new_symbols = symbols(new / "program.o")
assert old_symbols.keys() == new_symbols.keys()
root = macho.ROOT
assert old_symbols[root] == new_symbols[root]
next_entry = min((symbol for symbol in new_symbols if new_symbols[symbol] > new_symbols[root]),
                 key=new_symbols.__getitem__)
at = new_symbols[next_entry]
padding = (old_symbols[next_entry] - at) % 64
assert padding == 16
receipt = json.loads((new / "receipt.json").read_text())
result_size = trace.replace_atom(new / "program.o", output / "program.o", at, at,
                                 pack("<I", 0xd503201f) * (padding // 4))
assert result_size == receipt["object_text_bytes"] + padding == 71768
aligned_symbols = symbols(output / "program.o")
assert aligned_symbols.keys() == old_symbols.keys()
assert all(aligned_symbols[symbol] % 64 == old_symbols[symbol] % 64 for symbol in old_symbols)
(output / "metadata.json").write_bytes((new / "metadata.json").read_bytes())
runtime = json.loads((new / "runtime.json").read_text())
runtime["link"] = [arg.replace(str(new), str(output)) for arg in runtime["link"]]
runtime["diagnostic_object_postprocess"] = {
    "before": next_entry, "padding_bytes": padding, "input_object_sha256": sha(new / "program.o"),
    "object_text_symbol_residues_restored": len(aligned_symbols),
}
(output / "runtime.json").write_text(json.dumps(runtime, indent=2) + "\n")
subprocess.run(runtime["link"], check=True)
subprocess.run([sys.executable, "-B", q / "verify-worker.py", output / "mech-worker",
                output / "metadata.json", label], check=True)
control_worker = symbols(old / "mech-worker")
candidate_worker = symbols(output / "mech-worker")
same_alignment = [symbol for symbol in control_worker if control_worker[symbol] % 64 == candidate_worker[symbol] % 64]
natives = [symbol for symbol in control_worker if symbol.startswith("_mech_compact_")]
assert set(natives) <= set(same_alignment)
_, sections, _ = macho.load(output / "mech-worker")
receipt.update(
    object_text_bytes=result_size,
    object_text_x_roc=result_size / 57696,
    object_text_x_libdeflate=result_size / 37080,
    libdeflate_object_text_bytes=37080,
    object_sha256=sha(output / "program.o"),
    worker_sha256=sha(output / "mech-worker"),
    linked_text_bytes=sections[macho.TEXT][2],
    proof=str(q / f"{label}-qualification.json"),
    matching_linked_symbol_residues_mod_64=len(same_alignment),
    total_linked_symbols=len(control_worker),
    postprocess=runtime["diagnostic_object_postprocess"],
)
(output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps(receipt, indent=2), flush=True)
