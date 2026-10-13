# Experimental!
N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
Rotate      ← ↯⊃(⋅△)(⊏⊃(♭+⊃(⋅(×N⇡⧻))(+⊙(¤⇡N) ×⊃(¤<⊃⊢(⋅(⇡N)))(-⊃(⋅(¤⇡N))(◿⊃⊢(⊞+⊃(↘1)(⋅(⇡N)))))))(⋅♭))
Permutation ← ∧Rotate ⊃(≡⊂ Widths ◿(≡¤ Widths)⌊⊞÷ Divisors)(↯⊃(⊂⊃⧻(⋅N))(⋅(⇡N)))
FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))
FlipPermutations ← ↯⊃△ (⊏⊃(♭+⊃(×N⇡⧻)(FlipIndices ≡⊢))♭)
~FlipState {Permutations Counts}
FlipStep ← FlipState ⊃(FlipPermutations FlipState.Permutations|+⊃FlipState.Counts(>0≡⊢FlipState.Permutations))
Flips    ← FlipState.Counts ⍢(FlipStep|/↥>0≡⊢FlipState.Permutations) FlipState ⊃(∘|↯⊃(⧻|⋅0))
~F {Index End Checksum Maximum}
~Ranks {State Indices}
~Block {State Indices Counts}
BlockIndices ← +⊃F.Index(⇡↧1024 -⊃(F.Index|F.End))
Ranked       ← Ranks ⊃(∘|BlockIndices)
Candidate    ← Block ⊃(Ranks.State|Ranks.Indices|Flips Permutation Ranks.Indices) Ranked
Work         ← F ⊃(+⊃(F.Index Block.State)(⧻Block.Indices)|F.End Block.State|+⊃(F.Checksum Block.State)(/+×⊃(˜ⁿ¯1Block.Indices)Block.Counts)|↥⊃(F.Maximum Block.State)(/↥Block.Counts)) Candidate
Result       ← ≡°□ wait ≡spawn(□ ⍢(Work|<⊃F.End F.Index) F ⊃(⊢|⊣|⋅0|⋅0)) ⍉⊟ ⊃(↙↧2Total)(↘1) ⌊÷↧2Total ×Total⇡+1↧2Total
&p /+ ≡F.Checksum Result
&p $"Pfannkuchen(_) = _" N /↥ ≡F.Maximum Result
