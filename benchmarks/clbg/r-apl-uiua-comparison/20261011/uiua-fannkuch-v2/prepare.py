#!/usr/bin/env python3
"""Reconstruct additive Uiua native-array candidates without changing originals."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
ORIGINAL = SUITE / 'sources/uiua/fannkuchredux/fast.ua'
ORIGINAL_SHA = '3727af922300cbd313ae64b4bbcb29c4ed92a6ad967291f36e19daca84ecc9a3'


def main():
    assert hashlib.sha256(ORIGINAL.read_bytes()).hexdigest() == ORIGINAL_SHA
    text = ORIGINAL.read_text()
    assert text.count('FlipIndex   ← ⨬(⋅∘|˜-) ⊃(≤|∘|⋅∘)') == 1
    text = text.replace('Flip        ← ⍜↙⇌ +1⊸⊢\n', '')  # Unused in the original fast program.
    text = text.replace('FlipIndex   ← ⨬(⋅∘|˜-) ⊃(≤|∘|⋅∘)',
                        'FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))')
    text = text.replace('⊞FlipIndex ⊃(≡⊢FlipState.Permutations|⋅(⇡N))',
                        'FlipIndices ≡⊢FlipState.Permutations')
    (HERE / 'sources').mkdir(exist_ok=True)
    (HERE / 'sources/compact.ua').write_text(text)
    fast = text.replace('~FlipState {Permutations Counts}',
                        'FlipPermutations ← ↯⊃△ (⊏⊃(♭+⊃(×N⇡⧻)(FlipIndices ≡⊢))♭)\n~FlipState {Permutations Counts}')
    fast = fast.replace('≡⊏ ⊃(FlipIndices ≡⊢FlipState.Permutations)FlipState.Permutations',
                        'FlipPermutations FlipState.Permutations')
    (HERE / 'sources/fast.ua').write_text(fast)
    print('Prepared native-index and flat-gather candidates; originals unchanged. Validate before freezing.')


if __name__ == '__main__':
    main()
