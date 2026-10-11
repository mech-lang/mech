N                     ← ⋕◇∘⊡1 &args
Indices               ← ⇡⧻
Sums                  ← ˙⊞+ Indices &p ⊂ "COEFFICIENT-ROWS=" °⋕ ⧻ .
Triangle              ← ÷2 ×⊸+1 Sums
ForwardCoefficients   ← ÷⊙1 +1 +⊃Indices Triangle
TransposeCoefficients ← ÷⊙1 +1 ⍉+⊃Indices(⍉Triangle)
Forward               ← ≡/+× ⊃ForwardCoefficients¤ &p "FORWARD-PRODUCT"
Transpose             ← ≡/+× ⊃TransposeCoefficients¤ &p "TRANSPOSE-PRODUCT"
Step                  ← Transpose Forward
V                     ← ⍥Step 19 ↯N 1
U                     ← Step V
Fixed                 ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
