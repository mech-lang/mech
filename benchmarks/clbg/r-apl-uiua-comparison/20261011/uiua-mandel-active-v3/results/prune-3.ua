~M {R I Index Cx Cy Iter}
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
&p json Prune M [0.0 3.0 1.0 ¯2.0] [0.0 0.0 1.0 1.0] [1 4 7 11] [¯1.4 ¯0.9 ¯0.2 0.1] 0.25 7
