N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Live Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Live|M.Cx|M.Cy|+1M.Iter)
Mark    ← M ⊃(M.R|M.I|×⊃M.Live(≤4+⊃(˙×M.R)(˙×M.I))|M.Cx|M.Cy|M.Iter)
Iterate ← Mark Advance
Pack    ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
Row     ← &fwa "/dev/stdout" Pack M.Live ⍢(Iterate|×⊃(<50M.Iter)(/↥M.Live)) M (↯N 0) (↯N 0) (↯N 1) Cr ⊙0
&p "P4"
&p $"_ _" N N
≡Row -1 ×2÷N⇡N
