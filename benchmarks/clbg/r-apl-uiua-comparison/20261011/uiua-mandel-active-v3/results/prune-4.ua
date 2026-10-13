~M {R I Index Cx Cy Iter}
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
&p json Prune M [2.0 2.0000000000000004 1.9999999999999998] [0.0 0.0 0.0] [0 2 5] [¯1.5 ¯1.25 ¯0.875] 0.25 7
