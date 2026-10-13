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
~FlipRecorder {State Frames}
ToFlip   ← FlipRecorder ⊃(FlipStep FlipRecorder.State|FlipRecorder.Frames)
Capture  ← FlipRecorder ⊃(FlipRecorder.State|⊂⊃FlipRecorder.Frames(□ FlipRecorder.State))
Logged   ← Capture ToFlip
InitTrace ← FlipRecorder ⊃(∘|¤□) FlipState ⊃(∘|↯⊃(⧻|⋅0))
CountsOf ← FlipState.Counts FlipRecorder.State
~TraceBlock {Indices Recorder}
Flips ← ⍢(Logged|/↥>0≡⊢FlipState.Permutations FlipRecorder.State) InitTrace
~F {Index End Checksum Maximum Logs}
~Ranks {State Indices}
~Block {State Indices Counts}
BlockIndices ← +⊃F.Index(⇡↧1024 -⊃(F.Index|F.End))
Ranked       ← Ranks ⊃(∘|BlockIndices)
Candidate    ← Block ⊃(Ranks.State|Ranks.Indices|Flips Permutation Ranks.Indices) Ranked
Work         ← F ⊃(+⊃(F.Index Block.State)(⧻Block.Indices)|F.End Block.State|+⊃(F.Checksum Block.State)(/+×⊃(˜ⁿ¯1Block.Indices)(CountsOf Block.Counts))|↥⊃(F.Maximum Block.State)(/↥(CountsOf Block.Counts))|⊂⊃(F.Logs Block.State)(□ TraceBlock ⊃Block.Indices Block.Counts)) Candidate
Run ← ⍢(Work|<⊃F.End F.Index)
T ← ↧8 Total
Bounds ← ⌊÷T ×Total⇡+1T
Ranges ← ⍉⊟ ⊃(↙T|↘1) Bounds
Worker ← □ Run F ⊃(⊢|⊣|⋅0|⋅0|⋅[])
Handles ← ≡spawnWorker Ranges
Joined ← ≡°□ wait Handles
&p json Joined
