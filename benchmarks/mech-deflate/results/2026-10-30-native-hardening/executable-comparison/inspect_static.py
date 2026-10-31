"""Count static instruction sites; these are not executed instruction counts.

Usage: python3 inspect_static.py MECH_OBJECT REFERENCE_EXECUTABLE OUTPUT_JSON
Only the two text sections are disassembled. No program is executed or rebuilt.
"""

from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def inspect(path):
    command = ["xcrun", "llvm-objdump", "--disassemble", "--no-show-raw-insn",
               "--section=__text", str(path)]
    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             text=True)
    mnemonics = Counter()
    unrecognized = Counter()
    for line in child.stdout:
        match = re.match(r"^\s*([0-9a-f]+):\s+(\S+)", line)
        if match:
            mnemonic = match[2]
            if re.fullmatch(r"[a-z][a-z0-9.]*", mnemonic):
                mnemonics[mnemonic] += 1
            else:
                unrecognized[mnemonic] += 1
    error = child.stderr.read()
    assert child.wait() == 0, error
    assert sum(mnemonics.values()) > 0
    categories = Counter()
    for mnemonic, count in mnemonics.items():
        if mnemonic in {"bl", "blr", "blraa", "blrab"}:
            category = "call_sites"
        elif mnemonic.startswith("b.") or mnemonic in {"cbz", "cbnz", "tbz", "tbnz"}:
            category = "conditional_branch_sites"
        elif mnemonic in {"b", "br", "braa", "brab"}:
            category = "unconditional_branch_sites"
        elif mnemonic in {"ret", "retaa", "retab"}:
            category = "return_sites"
        elif mnemonic.startswith("ld"):
            category = "load_instruction_sites"
        elif mnemonic.startswith("st"):
            category = "store_instruction_sites"
        else:
            category = "other_instruction_sites"
        categories[category] += count
    return {"command": command, "stderr": error,
            "file_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "file_bytes": path.stat().st_size,
            "decoded_instruction_sites": sum(mnemonics.values()),
            "unrecognized_tokens": dict(sorted(unrecognized.items())),
            "categories": dict(sorted(categories.items())),
            "mnemonics": dict(sorted(mnemonics.items()))}


result = {"scope": "Static sites including padding and unreachable/error paths. Mech is its generated compressor entry; reference is its whole executable text. These are not dynamic calls, branch misses, bytes copied or hardware instructions retired.",
          "tool_version": subprocess.check_output(["xcrun", "llvm-objdump", "--version"], text=True),
          "mech": inspect(Path(sys.argv[1])),
          "reference": inspect(Path(sys.argv[2]))}
Path(sys.argv[3]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({label: {key: value for key, value in result[label].items()
                          if key not in {"mnemonics", "command"}}
                  for label in ["mech", "reference"]}, indent=2))
