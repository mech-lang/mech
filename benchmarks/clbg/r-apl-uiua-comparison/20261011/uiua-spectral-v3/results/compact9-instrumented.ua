N         ← ⋕◇∘⊡1 &args
Triangle  ← ÷2 ×⊸+1 ˙⊞+ ⇡⧻ &p ⊂ "COEFFICIENT-ROWS=" °⋕ ⧻ .
Forward   ← ≡/+× ⊃(÷⊙1 +1 +⊃(⇡⧻)Triangle)¤ &p "FORWARD-PRODUCT"
Transpose ← ≡/+× ⊃(÷⊙1 +1 ⍉+⊃(⇡⧻)(⍉Triangle))¤ &p "TRANSPOSE-PRODUCT"
Step      ← Transpose Forward
V         ← ⍥Step 19 ↯N 1
U         ← Step V
Fixed     ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
&p Fixed √ ÷(/+˙×V) /+×U V
