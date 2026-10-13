# Experimental!
Total ← 8193
~F {Index End Ranks}
BlockIndices ← +⊃F.Index(⇡↧1024 -⊃(F.Index|F.End))
Work ← F ⊃(+⊃F.Index(⧻BlockIndices)|F.End|⊂⊃F.Ranks BlockIndices)
Run ← ⍢(Work|<⊃F.End F.Index)
T ← ↧7 Total
Bounds ← ⌊÷T ×Total⇡+1T
Ranges ← ⍉⊟ ⊃(↙T|↘1) Bounds
Worker ← □ Run F ⊃(⊢|⊣|⋅[])
Handles ← ≡spawnWorker Ranges
Joined ← ≡°□ wait Handles
&p json Joined
