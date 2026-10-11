N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
Rotate      ← ⊂⊃(↻⊃(⊣|↙⊢)|↘⊢)
Permutation ← ∧Rotate ⊃(≡⊟ Widths ◿Widths⌊÷Divisors|⋅(⇡N))
FlipIndices ← +⊙(¤⇡N) ×⊃(⊞≤ ⊙(⇡N))(⊞˜- ⊙(×2⇡N))
~FlipState {Permutations Counts}
FlipStep ← FlipState ⊃(≡⊏ ⊃(FlipIndices ≡⊢FlipState.Permutations)FlipState.Permutations|+⊃FlipState.Counts(>0≡⊢FlipState.Permutations))
Flips    ← FlipState.Counts ⍢(FlipStep|/↥>0≡⊢FlipState.Permutations) FlipState ⊃(∘|↯⊃(⧻|⋅0))
~F {Index Checksum Maximum}
~Ranks {State Indices}
~Block {State Indices Counts}
BlockIndices ← +⊃F.Index(⇡↧1024 -⊃(F.Index|⋅Total))
Ranked       ← Ranks ⊃(∘|BlockIndices)
Candidate    ← Block ⊃(Ranks.State|Ranks.Indices|Flips≡Permutation Ranks.Indices) Ranked
Work         ← F ⊃(+⊃(F.Index Block.State)(⧻Block.Indices)|+⊃(F.Checksum Block.State)(/+×⊃(˜ⁿ¯1Block.Indices)Block.Counts)|↥⊃(F.Maximum Block.State)(/↥Block.Counts)) Candidate
Result       ← ⍢(Work|<Total F.Index) F 0 0 0
&p F.Checksum Result
&p $"Pfannkuchen(_) = _" N F.Maximum Result
