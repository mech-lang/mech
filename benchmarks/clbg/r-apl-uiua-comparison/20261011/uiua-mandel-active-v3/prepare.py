#!/usr/bin/env python3
"""Write an additive active-pixel source; do not execute or change frozen inputs."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
ORIGINAL = HERE.parent / 'uiua-mandelbrot-v2/sources/fast.ua'
ORIGINAL_SHA = '64f2913ec83009b70cc3ac8c4064438732bc1faf4448af198b7052e4cb0aaa4e'
SOURCE = '''N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Index Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Index|M.Cx|M.Cy|+1M.Iter)
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
Iterate ← Prune Advance
Bits ← ⍜⊏⋅1 ⊃M.Index(⋅↯N 0)
Pack ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
Row ← &fwa "/dev/stdout" Pack Bits ⍢(Iterate|×⊃(<50M.Iter)(>0⧻M.Index)) M (↯N 0) (↯N 0) (⇡N) Cr ⊙0
&p "P4"
&p $"_ _" N N
≡Row -1 ×2÷N⇡N
'''


def main():
    assert hashlib.sha256(ORIGINAL.read_bytes()).hexdigest() == ORIGINAL_SHA
    source = HERE / 'sources/fast.ua'
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(SOURCE)
    print(source, len(SOURCE.splitlines()), 'LOC', hashlib.sha256(source.read_bytes()).hexdigest())
    print('Prepared only; original qualified source, runtime, measurements and frozen audit unchanged.')


if __name__ == '__main__':
    main()
