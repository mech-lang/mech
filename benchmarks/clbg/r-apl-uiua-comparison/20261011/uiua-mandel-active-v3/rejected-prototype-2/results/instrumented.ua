# Experimental!
N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Index Cx Cy Iter}
Advance ← (
  M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Index|M.Cx|M.Cy|+1M.Iter)
  &p ⊂ "ADVANCED=" json .
  &p ⊂ "R_BYTES=" json ♭ bytes⌞ "f64" M.R .
  &p ⊂ "I_BYTES=" json ♭ bytes⌞ "f64" M.I .
)
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
Iterate ← (
  &p ⊂ "INPUT=" json .
  Prune Advance
  &p ⊂ "OUT=" json .
)
Bits ← ⍜⊏(⋅1) ⊃M.Index(⋅↯N 0)
Pack ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
Row ← &p ⊂ "ROW=" json Pack Bits ⍢(Iterate|×⊃(<50M.Iter)(>0⧻M.Index)) M (↯N 0) (↯N 0) (⇡N) Cr ⊙0
&p "P4"
&p $"_ _" N N
≡Row -1 ×2÷N⇡N
