# Experimental!
◌ ⍏ ◿16001 ×7919 ⇡16000
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
Run ← ⍢(Work|<⊃F.End F.Index)
T ← ↧8 Total
Bounds ← ⌊÷T ×Total⇡+1T
Ranges ← ⍉⊟ ⊃(↙T|↘1) Bounds
Worker ← (
  ◌ recv 0
  □ Run F ⊃(⊢|⊣|⋅0|⋅0)
)
Handles ← ≡spawnWorker Ranges
&p "READY"
◌ &fras "/dev/stdin"
send Handles 1
Joined ← ≡°□ wait Handles
&p /+ ≡F.Checksum Joined
&p $"Pfannkuchen(_) = _" N /↥ ≡F.Maximum Joined
