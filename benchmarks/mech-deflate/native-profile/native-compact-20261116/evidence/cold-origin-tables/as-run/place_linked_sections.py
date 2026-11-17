"""Isolate the linked stub/constant movement with the same section alignment in both arms."""

import importlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
macho = importlib.import_module("reachability-census")
shared = ("__stubs", "__stub_helper", "__const", "__cstring")
setup = {
    "cold-origin-page-control": q / "native-size-20261117/source-aligned",
    "cold-origin-page-candidate": q / "cold-origin-packed-native/final-candidate",
}
option = "link-arg=-Wl,-sectalign,__TEXT,__stubs,0x1000"
placements = {}
environment = {name: value for name, value in os.environ.items() if not name.startswith("MECH_")}
for name, source in setup.items():
    target = q / f"{name}-native/final-candidate"
    target.mkdir(parents=True)
    for name_in in ("program.o", "metadata.json"):
        shutil.copyfile(source / name_in, target / name_in)
    old = json.loads((source / "runtime.json").read_text())
    link = [arg.replace(str(source / "program.o"), str(target / "program.o"))
            .replace(str(source / "mech-worker"), str(target / "mech-worker")) for arg in old["link"]]
    assert link.count(str(target / "mech-worker")) == 1
    link.extend(("-C", option))
    (target / "runtime.json").write_text(json.dumps(old | {
        "link": link, "diagnostic_link_experiment": "Same __TEXT,__stubs alignment for both workers.",
    }, indent=2) + "\n")
    subprocess.run(link, check=True, env=environment)
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", target / "mech-worker",
                    target / "metadata.json", name], check=True, env=environment)
    _, sections, symbols = macho.load(target / "mech-worker")
    native = macho.text_symbols(sections, symbols)
    placements[name] = {
        "sections": {section.decode(): [address, size, offset]
                     for (segment, section), (_, address, size, offset, _, _) in sections.items()
                     if segment == b"__TEXT"},
        "root": native[macho.ROOT],
        "file_bytes": (target / "mech-worker").stat().st_size,
    }
first, second = placements.values()
(study / "aligned-link-placements.json").write_text(json.dumps(placements, indent=2) + "\n")
same = [name for name in shared if first["sections"][name] == second["sections"][name]]
assert same == list(shared), same
assert first["root"] == second["root"]
print("The same link settings preserve stubs, constants, strings and checked-root placement.")
