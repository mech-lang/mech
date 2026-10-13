# Experimental!
N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
Rotate      ← ⊂⊃(↻⊃(⊣|↙⊢)|↘⊢)
Permutation ← ∧Rotate ⊃(≡⊟ Widths ◿Widths⌊÷Divisors|⋅(⇡N))
FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))
FlipPermutations ← ↯⊃△ (⊏⊃(♭+⊃(×N⇡⧻)(FlipIndices ≡⊢))♭)
~FlipState {Permutations Counts}
FlipStep ← FlipState ⊃(FlipPermutations FlipState.Permutations|+⊃FlipState.Counts(>0≡⊢FlipState.Permutations))
Flips    ← FlipState.Counts ⍢(FlipStep|/↥>0≡⊢FlipState.Permutations) FlipState ⊃(∘|↯⊃(⧻|⋅0))
&p json ≡Permutation [0]
