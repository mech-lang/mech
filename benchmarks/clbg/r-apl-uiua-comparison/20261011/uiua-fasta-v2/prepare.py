#!/usr/bin/env python3
"""Prepare an additive, bounded Uiua FASTA output candidate without execution."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
ORIGINAL = SUITE / 'sources/uiua/fasta/fast.ua'
ORIGINAL_SHA = 'f0dd51cda0e6dbae84afd0e95d7c4ceeb4aaac6d3892824a413926ab54c5cf05'
OLD_WRAP = 'Wrap  ← ≡◇&p ⊕□⊸(⌊÷60⇡⧻)'
NEW_WRAP = '''~Wrapped {Chars Indices}
Indexed   ← Wrapped ⊃(∘|⇡+⊃⧻(⌈÷60⧻))
CharIndex ← -⊃(⌊÷61Wrapped.Indices)Wrapped.Indices
LineEnd   ← ↥⊃(=0◿61+1Wrapped.Indices)(=⊃Wrapped.Indices(-1⧻Wrapped.Indices))
Packed    ← ⊏⊃(+⊃CharIndex(×⊃LineEnd(-⊃CharIndex(⧻Wrapped.Chars))))(⊂⊃Wrapped.Chars(⋅"\\n")) Indexed
Wrap      ← &w⊃Packed(⋅1)'''
ALU = '''AluDraw ← ⊏⊃(◿(⧻Alu)+⊃G.Seed(⇡Size))(⋅Alu)
AluAdvance ← (
  ⟜(Wrap AluDraw)
  G ⊃(+⊃G.Seed Size|-⊃Size G.Left|G.Letters|G.Cdf)
)'''
OLD_ONE = 'Wrap ↯[×2N] Alu'
NEW_ONE = '◌ ⍢(AluAdvance|>0G.Left) G 0 (×2N) Alu []'


def main():
    assert hashlib.sha256(ORIGINAL.read_bytes()).hexdigest() == ORIGINAL_SHA
    text = ORIGINAL.read_text()
    assert text.count(OLD_WRAP) == text.count(OLD_ONE) == 1
    assert text.count('Generate ← G.Seed ⍢(Advance|>0G.Left)') == 1
    updated = text.replace(OLD_WRAP, NEW_WRAP).replace(OLD_ONE, NEW_ONE)
    updated = updated.replace('Generate ← G.Seed ⍢(Advance|>0G.Left)',
                              'Generate ← G.Seed ⍢(Advance|>0G.Left)\n' + ALU)
    source = HERE / 'sources/fast.ua'
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(updated)
    print(source, sum(bool(line.strip()) for line in updated.splitlines()),
          hashlib.sha256(source.read_bytes()).hexdigest())
    print('Prepared only; no Uiua execution, validation, timings or frozen changes.')


if __name__ == '__main__':
    main()
