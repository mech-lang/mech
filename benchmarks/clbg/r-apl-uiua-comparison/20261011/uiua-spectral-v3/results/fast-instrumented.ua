N                     ← ⋕◇∘⊡1 &args
~Product {Vector Offset}
Starts                ← ×128⇡⌈÷128N
Rows                  ← +⊃Product.Offset(⇡↧128 -⊃Product.Offset(⋅N))
Sums                  ← ⊞+ ⊃Rows(⋅(⇡N)) &p ⊂ "COEFFICIENT-ROWS=" °⋕ ⧻ Rows .
Triangle              ← ÷2 ×⊸+1 Sums
ForwardCoefficients   ← ÷⊙1 +1 +⊃Rows Triangle
TransposeCoefficients ← ÷⊙1 +1 ⍉+⊃(⋅(⇡N))(⍉Triangle)
ForwardBlock          ← ≡/+× ⊃ForwardCoefficients(¤Product.Vector)
TransposeBlock        ← ≡/+× ⊃TransposeCoefficients(¤Product.Vector)
Forward               ← /◇⊂ ≡(□ForwardBlock Product) ¤ ⊙Starts &p "FORWARD-PRODUCT"
Transpose             ← /◇⊂ ≡(□TransposeBlock Product) ¤ ⊙Starts &p "TRANSPOSE-PRODUCT"
Step                  ← Transpose Forward
V                     ← ⍥Step 19 ↯N 1
U                     ← Step V
Fixed                 ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
