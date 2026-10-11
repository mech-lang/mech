N ← ↥6 ⋕◇∘⊡1 &args
~Tree {Id Left Right}
Leaf   ← Tree ⊃(⊣|⋅[]|⋅[])
Build  ← |1 ⨬(Tree ⊃(⊣|Build ⊃[-1⊢|×2⊣]|Build ⊃[-1⊢|+1×2⊣])|Leaf) ⊸(=0⊢)
Check  ← |1 ⨬(+1+⊃(Check Tree.Left)(Check Tree.Right)|⋅1) ⊸(=0⧻Tree.Left)
TreeAt ← Build ⊟⊙1
&p $"stretch tree of depth _\t check: _" ⟜(Check TreeAt) +1N
Long       ← TreeAt N
Depths     ← +4×2⇡+1⌊÷2 -4N
Iteration  ← +⊃(Check TreeAt)(⋅∘)
Iterations ← ˜ⁿ2-⊙(+4N)
SumChecks  ← ∧Iteration ⊃(↯⊃Iterations∘|⋅0)
Batch      ← &p $"_\t trees of depth _\t check: _" ⊃(Iterations|∘|SumChecks)
≡Batch Depths
&p $"long lived tree of depth _\t check: _" N Check Long
