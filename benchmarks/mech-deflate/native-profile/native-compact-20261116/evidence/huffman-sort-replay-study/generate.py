"""Extract current sorting functions for a separate identical-input replay.

This does not edit the codec. The bucket alternative reuses its existing
count/cumulative/place/heap helpers, with the same packed output order.
"""
import argparse
from hashlib import sha256
import json
from pathlib import Path
import re

SOURCE = "653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299"
PRIMITIVES = ["huff-word", "huff-packed", "huff-minimum", "huff-get32", "huff-get64", "huff-set32", "huff-set64"]
SHELL = ["huff-pack-active-symbols", "huff-insert-gapped-key", "huff-sort-key-gap",
         "huff-sort-key-gaps", "huff-packed-sort-symbols"]
BUCKET = ["huff-count-active", "huff-cumulative-step", "huff-cumulative", "huff-place-nonzero",
    "huff-sift-child", "huff-sift-descend", "huff-sift-loop", "huff-heapify", "huff-heap-extract",
    "huff-heap-sort-at", "huff-heap-sort", "huff-packed-count-step", "huff-packed-count-symbols",
    "huff-packed-place-step", "huff-packed-place-symbols"]


def extract(source, name):
    begin = re.search(r"(?m)^" + re.escape(name) + r"\(", source).start()
    header_end = source.index("\n", begin) + 1
    end = re.search(r"(?m)^[a-z][a-z0-9-]*\(", source[header_end:])
    assert end
    return source[begin:header_end+end.start()].rstrip() + "\n\n"


def generate(source, role):
    assert sha256(source.encode()).hexdigest() == SOURCE
    names = PRIMITIVES + (SHELL if role == "shell" else BUCKET)
    body = "+> bits\n+> sequence\n\n" + "".join(extract(source, n) for n in names)
    body += "replay(freqs<[u32]>) => (out<([u32],u64)>) :=\n"
    body += "  n := sequence/length(freqs)\n  scratch := [0u32 | i <- 0u64..=n, i < n]\n"
    if role == "shell":
        body += "  out := huff-packed-sort-symbols(n, freqs, 0u64, scratch).\n"
    else:
        body += """  top := n - 1u64
  counters := [0u64 | i <- 0u64..=n, i < n]
  counted := huff-packed-count-symbols(freqs, 0u64, n, counters, 0u64, top)
  (cumulative, used) := huff-cumulative(counted, n, 0u64, 1u64)
  (placed, words) := huff-packed-place-symbols(freqs, 0u64, n, cumulative, scratch, 0u64, top)
  base := huff-get64(placed, n - 2u64)
  size := huff-get64(placed, top) - base
  sorted := huff-heap-sort(words, base, size)
  out := (sorted, used).
"""
    return body + "\nreplay(freqs<[u32]>)\n"


def main():
    p = argparse.ArgumentParser(__doc__)
    p.add_argument("source", type=Path)
    p.add_argument("output", type=Path)
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    pins = {}
    for role in ("shell", "bucket"):
        source = generate(args.source.read_text(), role)
        (args.output / (role + ".mec")).write_text(source)
        pins[role] = {"sha256": sha256(source.encode()).hexdigest(), "lines": len(source.splitlines())}
    (args.output / "source-pins.json").write_text(json.dumps(pins, indent=2) + "\n")
    print(json.dumps(pins, indent=2))


if __name__ == "__main__":
    main()
