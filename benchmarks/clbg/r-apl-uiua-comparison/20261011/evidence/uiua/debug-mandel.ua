N ← ⋕◇⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Live Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Live|M.Cx|M.Cy|+1M.Iter)
Mark ← M ⊃(M.R|M.I|×⊃M.Live(≤4+⊃(˙×M.R)(˙×M.I))|M.Cx|M.Cy|M.Iter)
Iterate ← Mark Advance

&p M.Live ⍥Iterate 50 M (↯N 0) (↯N 0) (↯N 1) Cr ¯1 0
&p Cr
