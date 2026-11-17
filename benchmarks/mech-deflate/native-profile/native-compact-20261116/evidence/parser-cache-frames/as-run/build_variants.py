"""Prepare parser probes before any timing trials."""

import sys
from study_tools import build, link, probe, sha, study, write

for role in sys.argv[1:]:
    out = study / role
    library, rustc = build(out, out / "source", f"queue-study-{role}")
    exe, linker = link(out, library)
    check = probe(exe, out / "correctness.json")
    write(out / "build-receipt.json", {"role": role, "build": rustc, "link": linker,
                                      "library_sha256": sha(library), "probe_sha256": sha(exe),
                                      "exact_tree_and_work": check})
    print(role, "same tree, events and steps", flush=True)
