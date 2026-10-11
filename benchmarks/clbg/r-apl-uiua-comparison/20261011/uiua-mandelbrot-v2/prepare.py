#!/usr/bin/env python3
"""Change only the counted Uiua Pack expression; retain frozen originals."""
from pathlib import Path
import hashlib
H=Path(__file__).resolve().parent
N=H.parent
OLD=N/'sources/uiua/mandelbrot/fast.ua'
PIN='3f3f1a02e405e93ac2e3e90298d45e1a34d6ab21714b96ab2bb3b779c62dcd57'
A='Pack    ← ≡(/+×⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8'
B='Pack    ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8'
assert hashlib.sha256(OLD.read_bytes()).hexdigest()==PIN
text=OLD.read_text()
assert text.count(A)==1
(H/'sources').mkdir(exist_ok=True)
(H/'sources/fast.ua').write_text(text.replace(A,B))
print('Native Pack candidate prepared; all recurrence, escape and output expressions unchanged.')
