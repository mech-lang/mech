N           ← ⋕◇∘⊡1 &args
Widths      ← ⇌+2⇡-1N
Divisors    ← ⇌↙-1N \×+1⇡N
Total       ← /×+1⇡N
Rotate      ← ⊂⊃(↻⊃(⊣|↙⊢)|↘⊢)
Permutation ← ∧Rotate ⊃(≡⊟ Widths ◿Widths⌊÷Divisors|⋅(⇡N))
Flip        ← ⍜↙⇌ +1⊸⊢
~FlipState {Permutation Count}
FlipStep ← FlipState ⊃(Flip FlipState.Permutation|+1FlipState.Count)
Flips    ← FlipState.Count ⍢(FlipStep|>0⊢FlipState.Permutation) FlipState ⊙0
~F {Index Checksum Maximum}
~Run {State Count}
Candidate ← Run ⊃(∘|Flips Permutation F.Index)
Work ← (
  Candidate
  F ⊃(+1F.Index Run.State|+⊃(×⊃(˜ⁿ¯1 F.Index Run.State)Run.Count)(F.Checksum Run.State)|↥⊃Run.Count(F.Maximum Run.State))
)
Result ← ⍥Work Total F 0 0 0
&p F.Checksum Result
&p $"Pfannkuchen(_) = _" N F.Maximum Result
