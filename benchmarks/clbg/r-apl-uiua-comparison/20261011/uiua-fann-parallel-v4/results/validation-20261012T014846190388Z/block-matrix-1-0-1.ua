# Experimental!
N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
~Rotation {Width Amounts}
ColumnMask         ← ¤ <⊃Rotation.Width(⋅(⇡N))
Shifted            ← ◿⊃Rotation.Width(⊞+⊃Rotation.Amounts(⋅(⇡N)))
RotationIndices    ← +⊙(¤⇡N) ×⊃ColumnMask(-⊃(⋅(¤⇡N))Shifted)
~RotateWork {Stage Permutations}
RotateGather       ← ↯⊃(△RotateWork.Permutations)(⊏⊃(♭+⊃(×N⇡⧻ RotateWork.Permutations)(RotationIndices RotateWork.Stage))(♭RotateWork.Permutations))
RotateRows         ← RotateGather RotateWork
Rotations          ← ≡Rotation Widths ◿(≡¤ Widths)⌊⊞÷ Divisors
InitialPermutations ← ↯⊃(⊂⊃⧻(⋅N))(⋅(⇡N))
PermutationBlock   ← ∧RotateRows ⊃Rotations InitialPermutations
FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))
FlipPermutations ← ↯⊃△ (⊏⊃(♭+⊃(×N⇡⧻)(FlipIndices ≡⊢))♭)
~FlipState {Permutations Counts}
FlipStep ← FlipState ⊃(FlipPermutations FlipState.Permutations|+⊃FlipState.Counts(>0≡⊢FlipState.Permutations))
Flips    ← FlipState.Counts ⍢(FlipStep|/↥>0≡⊢FlipState.Permutations) FlipState ⊃(∘|↯⊃(⧻|⋅0))
&p json PermutationBlock [0]
