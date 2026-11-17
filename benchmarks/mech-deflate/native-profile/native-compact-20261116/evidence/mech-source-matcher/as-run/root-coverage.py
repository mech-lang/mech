"""Count joint stale roots of the pinned input using the Mech insert/slide rules."""

from collections import Counter
from hashlib import sha256
import json
from pathlib import Path

study = Path(__file__).resolve().parent
q = study.parent
names = (
    "random-64", "random-1024", "random-32769",
    "prefix-random-4096", "prefix-random-8192", "prefix-random-32768", "prefix-random-1048576",
    "prefix-natural-4096", "prefix-natural-8192", "prefix-natural-32768",
    "prefix-repeated-32769", "prefix-periodic-32768", "heldout-random-65537",
)
fixtures = {f["name"]: f for f in json.loads((q / "fixtures.json").read_text())}
assert set(names) <= fixtures.keys()
window = 32768


def hash16(value):
    return (value * 506832829 >> 16) & 65535


def coverage(data):
    # An absolute position is equivalent to a live U16 node plus its
    # current base, with the zero sentinels represented by -window.
    roots3 = [-window] * 65536
    roots4 = [-window] * 65536
    lengths = Counter()
    possible = Counter()
    joint = single3 = single4 = 0
    start = 0
    run = 0
    steps = max(0, len(data) - 4)
    for i in range(steps):
        sequence = int.from_bytes(data[i:i+4], "little") if i else 0
        h3 = hash16(sequence & 16777215)
        h4 = hash16(sequence)
        stale3 = i - roots3[h3] >= window
        stale4 = i - roots4[h4] >= window
        roots3[h3] = i
        roots4[h4] = i
        single3 += stale3
        single4 += stale4
        eligible = stale3 and stale4
        joint += eligible
        if (i % window == 0 or not eligible) and run:
            lengths[run] += 1
            for stride in (4, 8, 16, 32):
                wait = (-start) % stride
                possible[stride] += max(0, run - wait)
            run = 0
        # The ordinary collector must execute the slide at this position.
        if eligible and (i == 0 or i % window != 0):
            if not run:
                start = i
            run += 1
    if run:
        lengths[run] += 1
        for stride in (4, 8, 16, 32):
            possible[stride] += max(0, run - ((-start) % stride))
    return {
        "positions": steps, "stale_short_root": single3, "stale_four_byte_root": single4, "joint_stale": joint,
        "contiguous_runs_separated_at_slide": dict(sorted(lengths.items())),
        "admission_coverage_upper_bound": dict(possible),
    }


result = {}
for case in names:
    data = Path(fixtures[case]["input"]).read_bytes()
    result[case] = {"input_sha256": sha256(data).hexdigest(), **coverage(data)}
    print(case, result[case]["positions"], result[case]["joint_stale"],
          result[case]["admission_coverage_upper_bound"], flush=True)
(study / "root-coverage.json").write_text(json.dumps({
    "scope": "Exact root-table updates from the pinned Mech source. Coverage also depends on matcher skip, observations, cache and checkpoint limits.",
    "cases": result,
}, indent=2) + "\n")
