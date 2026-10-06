#!/usr/bin/env python3
"""Derive a two-body case from the maintained application and a scalar oracle."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "audit/v04/site/fixtures/nbody-two"
DEST.mkdir(parents=True, exist_ok=True)
source = (ROOT / "examples/resident-n-body/n-body.mec").read_text()
source = source.replace("planets := [☉ ☿ ♀ ♁ ♂ ♃ ♄ ♅ ♆ ♇]'", "planets := [0 0 0 0 0 0 1; 1 0 0 0 1 0 0.001]").replace("n-choose-k(1..=10,2)", "n-choose-k(1..=2,2)")
config = (ROOT / "examples/resident-n-body/mech.mcfg").read_text()
(DEST / "n-body.mec").write_text(source)
(DEST / "mech.mcfg").write_text(config)
traces = {}
for dt in (0.01, 0.02):
    x = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
    v = [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    mass = [1.0, 0.001]
    states = [{"turn": 0, "positions": [row[:] for row in x]}]
    for turn in range(1, 11):
        delta = [x[0][axis] - x[1][axis] for axis in range(3)]
        magnitude = dt * sum(d * d for d in delta) ** -1.5
        for axis in range(3):
            v[0][axis] -= delta[axis] * mass[1] * magnitude
            v[1][axis] += delta[axis] * mass[0] * magnitude
        for body in range(2):
            for axis in range(3):
                x[body][axis] += v[body][axis] * dt
        states.append({"turn": turn, "positions": [row[:] for row in x]})
    traces[str(dt)] = states
assert traces["0.01"][1]["positions"] == [[1.0000000000000001e-07, 0.0, 0.0], [0.9999, 0.01, 0.0]]
reference = {"source_commit": "c4777b7015fe8ff47fdfa48d18606c49aace7d97", "derived_from": "examples/resident-n-body/n-body.mec", "source_sha256": hashlib.sha256(source.encode()).hexdigest(), "method": "Independent scalar two-body semi-implicit Euler recurrence; explicit initial positions, velocities, masses; same pair-update order. Not used to produce application output.", "absolute_tolerance": 1e-12, "traces": traces}
(DEST / "reference.json").write_text(json.dumps(reference, indent=2) + "\n")
