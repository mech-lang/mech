# Experimental!
N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
Rotate      ← ↯⊃(⋅△)(⊏⊃(♭+⊃(⋅(×N⇡⧻))(+⊙(¤⇡N) ×⊃(¤<⊃⊢(⋅(⇡N)))(-⊃(⋅(¤⇡N))(◿⊃⊢(⊞+⊃(↘1)(⋅(⇡N)))))))(⋅♭))
Permutation ← ∧Rotate ⊃(≡⊂ Widths ◿(≡¤ Widths)⌊⊞÷ Divisors)(↯⊃(⊂⊃⧻(⋅N))(⋅(⇡N)))
FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))
~FlipState {Permutations Counts}
FlipStep ← FlipState ⊃(≡⊏ ⊃(FlipIndices ≡⊢FlipState.Permutations)FlipState.Permutations|+⊃FlipState.Counts(>0≡⊢FlipState.Permutations))
Flips    ← FlipState.Counts ⍢(FlipStep|/↥>0≡⊢FlipState.Permutations) FlipState ⊃(∘|↯⊃(⧻|⋅0))
&p json Permutation [0]
