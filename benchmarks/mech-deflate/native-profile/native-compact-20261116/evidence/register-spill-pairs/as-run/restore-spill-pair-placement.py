"""Restore the original object and link placement after pairing the matcher spills."""
from hashlib import file_digest
from importlib import import_module
import json
from pathlib import Path
from struct import pack
import subprocess
import sys

q = Path(__file__).resolve().parent
study = q / "runtime-matchfinder-20261117"
base = q / "cranelift-width-16-owners-cold-native/final-candidate"
paired = q / "spill-pairs-20261117-native/final-candidate"
label = "spill-pairs-restored-placement"
output = q / f"{label}-native/final-candidate"
output.mkdir(parents=True)
trace = import_module("count-matcher-instructions")
macho = trace.macho


def sha(path):
    with path.open("rb") as f:
        return file_digest(f, "sha256").hexdigest()


def text_symbols(path):
    _, sections, table = macho.load(path)
    return macho.text_symbols(sections, table)


path = paired / "program.o"
before = base / path.name
prior_receipt = json.loads((base / "receipt.json").read_text())
pair_receipt = json.loads((paired / "receipt.json").read_text())
assert sha(path) == pair_receipt["object_sha256"]
assert sha(before) == prior_receipt["object_sha256"]
saved = prior_receipt["object_text_bytes"] - pair_receipt["object_text_bytes"]
data, sections, table = macho.load(path)
symbols = macho.text_symbols(sections, table)
section = sections[macho.TEXT]
assert section[1] == 0
start = symbols[trace.symbol]
limit = min(at for at in symbols.values() if at > start)
assert not any(start <= at < limit for at, _ in trace.layout.obj.relocs(data, section))
assert saved % 16 == 0 and saved > 0
original = data[section[3]+start:section[3]+limit]
trace.replace_atom(path, output/"program.o", start, limit, original + pack("<I",0xd503201f)*(saved//4))
assert text_symbols(output/"program.o") == text_symbols(before)
(output/"metadata.json").write_bytes((paired/"metadata.json").read_bytes())
runtime = json.loads((paired/"runtime.json").read_text())
runtime["link"] = [arg.replace(str(paired),str(output)) for arg in runtime["link"]]
runtime["diagnostic_object_postprocess"] = {
    "matched_atom": trace.symbol, "bytes_of_unreachable_padding": saved,
    "paired_object_sha256": pair_receipt["object_sha256"],
    "all_object_text_symbol_addresses_restored": True,
}
subprocess.run(runtime["link"],check=True)
subprocess.run([sys.executable,"-B",q/"verify-worker.py",output/"mech-worker",output/"metadata.json",
                f"{label}-worker"],check=True)
control_worker, candidate_worker = (text_symbols(folder/"mech-worker") for folder in (base,output))
native_symbols = [symbol for symbol in control_worker if symbol.startswith("_mech_compact_")]
moved = [symbol for symbol in native_symbols if control_worker[symbol] != candidate_worker.get(symbol)]
_, final_sections, _ = macho.load(output/"program.o")
receipt = pair_receipt | {
    "object_text_bytes": final_sections[macho.TEXT][2],
    "object_text_x_roc": final_sections[macho.TEXT][2] / 57696,
    "object_text_x_libdeflate": final_sections[macho.TEXT][2] / 37080,
    "libdeflate_object_text_bytes": 37080,
    "object_sha256": sha(output/"program.o"),
    "metadata_sha256": sha(output/"metadata.json"),
    "worker_sha256": sha(output/"mech-worker"),
    "proof": str(q/f"{label}-worker-qualification.json"),
    "postprocess":runtime["diagnostic_object_postprocess"],
    "linked_native_symbol_addresses_checked":len(native_symbols),
    "linked_native_symbols_with_different_addresses": moved,
}
assert receipt["object_text_bytes"] == prior_receipt["object_text_bytes"]
(output/"receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
(output/"runtime.json").write_text(json.dumps(runtime,indent=2)+"\n")
plan = {
    "purpose": "Distinguish shifting later functions from changes within the paired matcher.",
    "runs": ["both complete orders of the focused list"],
    "cases": ["prefix-natural-65535", "prefix-natural-65536", "prefix-natural-65537",
              "prefix-natural-262144", "prefix-natural-8192", "prefix-random-4096",
              "prefix-random-1048576", "seven-stored-prefix", "random-64"],
    "symbols_moved_relative_to_control":moved,
    "control_worker_sha256":sha(base/"mech-worker"),
    "paired_worker_sha256":sha(paired/"mech-worker"),
    "restored_worker_sha256":receipt["worker_sha256"],
    "bootstrap":"same 0.5% resolution and 90% paired intervals as the original screen",
}
(study/"placement-cases.json").write_text(json.dumps(plan["cases"],indent=2)+"\n")
(study/"placement-plan.json").write_text(json.dumps(plan,indent=2)+"\n")
print(json.dumps({"placed_text_bytes":receipt["object_text_bytes"],
                  "restored_linked_native_symbols":len(native_symbols)-len(moved),
                  "linked_native_symbols_with_different_addresses":moved}))
