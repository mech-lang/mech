#!/usr/bin/env python3
"""Prepare one additive native-group candidate without executing a workload."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
ORIGINAL = HERE.parent / 'sources/uiua/knucleotide/fast.ua'


def main():
    original = ORIGINAL.read_text()
    assert hashlib.sha256(original.encode()).hexdigest() == 'b3606cf00816ea8152bc2a05f1d1cd2a89bf689a60c75f106e2e279e6d9dad38'
    old_add = 'AddCount ← insert ⊃(∘|+1⬚0get|⋅∘)'
    new_add = '\n'.join([
        '~Weighted {Key Weight}',
        'AddCounts ← insert ⊃(Weighted.Key|+⊃Weighted.Weight(⬚0get ⊃Weighted.Key(⋅∘))|⋅∘)',
        'ClassCounts ← ⊃(⊕⊢)(⊕⧻) ⊸⊛',
    ])
    old_count = 'ChunkCounts ← ∧AddCount ⊃ChunkWindows K.Counts'
    new_count = 'ChunkCounts ← ∧AddCounts ⊃(≡Weighted ClassCounts ChunkWindows)K.Counts'
    assert original.count(old_add) == original.count(old_count) == 1
    source = original.replace(old_add, new_add).replace(old_count, new_count)
    destination = HERE / 'sources/fast.ua'
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(source)
    print(destination, hashlib.sha256(source.encode()).hexdigest())


if __name__ == '__main__':
    main()
