N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Index Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Index|M.Cx|M.Cy|+1M.Iter)
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
Iterate ← Prune Advance
Bits ← ⍜⊏(⋅1) ⊃M.Index(⋅(↯N 0))
Pack ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
Row ← Pack Bits ⍢(Iterate|×⊃(<50M.Iter)(>0⧻M.Index)) M (↯N 0) (↯N 0) (⇡N) Cr ⊙0
T ← ↧8N
Groups ← ⊕□⊸(◿T)⇡N
Worker ← □ ≡Row -1 ×2÷N °□
Handles ← ≡spawnWorker Groups
Order ← /◇⊂ Groups
&p "P4"
&p $"_ _" N N
&fwa "/dev/stdout" ⊏ (⍏Order) /◇⊂ wait Handles
