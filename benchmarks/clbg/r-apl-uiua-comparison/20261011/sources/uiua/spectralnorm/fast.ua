N         ← ⋕◇∘⊡1 &args
A         ← ÷⊙1 +1 +⟜(÷2×⊸+1+)
~Product {Vector Offset}
Starts ← ×128⇡⌈÷128N
Rows ← +⊃Product.Offset(⇡↧128 -⊃Product.Offset(⋅N))
ForwardCoefficients ← ⊞A ⊃Rows(⋅(⇡N))
TransposeCoefficients ← ⊞˜A ⊃Rows(⋅(⇡N))
ForwardBlock ← ≡/+× ⊃ForwardCoefficients(¤Product.Vector)
TransposeBlock ← ≡/+× ⊃TransposeCoefficients(¤Product.Vector)
Forward ← /◇⊂ ≡(□ForwardBlock Product) ¤ ⊙Starts
Transpose ← /◇⊂ ≡(□TransposeBlock Product) ¤ ⊙Starts
Step      ← Transpose Forward
V         ← ⍥Step 19 ↯N 1
U         ← Step V
Fixed     ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
