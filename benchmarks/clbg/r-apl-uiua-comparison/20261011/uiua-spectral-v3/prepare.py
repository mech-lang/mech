#!/usr/bin/env python3
"""Write only the additive vectorized spectral sources; no runtime execution."""
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCES = {
    'compact9': '''N         ← ⋕◇∘⊡1 &args
Triangle  ← ÷2 ×⊸+1 ˙⊞+ ⇡⧻
Forward   ← ≡/+× ⊃(÷⊙1 +1 +⊃(⇡⧻)Triangle)¤
Transpose ← ≡/+× ⊃(÷⊙1 +1 ⍉+⊃(⇡⧻)(⍉Triangle))¤
Step      ← Transpose Forward
V         ← ⍥Step 19 ↯N 1
U         ← Step V
Fixed     ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
''',
    'compact': '''N                     ← ⋕◇∘⊡1 &args
Indices               ← ⇡⧻
Sums                  ← ˙⊞+ Indices
Triangle              ← ÷2 ×⊸+1 Sums
ForwardCoefficients   ← ÷⊙1 +1 +⊃Indices Triangle
TransposeCoefficients ← ÷⊙1 +1 ⍉+⊃Indices(⍉Triangle)
Forward               ← ≡/+× ⊃ForwardCoefficients¤
Transpose             ← ≡/+× ⊃TransposeCoefficients¤
Step                  ← Transpose Forward
V                     ← ⍥Step 19 ↯N 1
U                     ← Step V
Fixed                 ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
''',
    'fast': '''N                     ← ⋕◇∘⊡1 &args
~Product {Vector Offset}
Starts                ← ×128⇡⌈÷128N
Rows                  ← +⊃Product.Offset(⇡↧128 -⊃Product.Offset(⋅N))
Sums                  ← ⊞+ ⊃Rows(⋅(⇡N))
Triangle              ← ÷2 ×⊸+1 Sums
ForwardCoefficients   ← ÷⊙1 +1 +⊃Rows Triangle
TransposeCoefficients ← ÷⊙1 +1 ⍉+⊃(⋅(⇡N))(⍉Triangle)
ForwardBlock          ← ≡/+× ⊃ForwardCoefficients(¤Product.Vector)
TransposeBlock        ← ≡/+× ⊃TransposeCoefficients(¤Product.Vector)
Forward               ← /◇⊂ ≡(□ForwardBlock Product) ¤ ⊙Starts
Transpose             ← /◇⊂ ≡(□TransposeBlock Product) ¤ ⊙Starts
Step                  ← Transpose Forward
V                     ← ⍥Step 19 ↯N 1
U                     ← Step V
Fixed                 ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
''',
}


def prepare():
    directory = HERE / 'sources'
    directory.mkdir(parents=True, exist_ok=True)
    for variant, source in SOURCES.items():
        (directory / (variant + '.ua')).write_text(source)


if __name__ == '__main__':
    prepare()
