# Experimental!
N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Index Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Index|M.Cx|M.Cy|+1M.Iter)
~Selection {State Keep}
Selected ← Selection ⊃(∘|⊚ ≤4+⊃(˙×M.R)(˙×M.I))
Prune ← M ⊃(⊏⊃Selection.Keep(M.R Selection.State)|⊏⊃Selection.Keep(M.I Selection.State)|⊏⊃Selection.Keep(M.Index Selection.State)|⊏⊃Selection.Keep(M.Cx Selection.State)|M.Cy Selection.State|M.Iter Selection.State) Selected
Iterate ← Prune Advance
Bits ← ⍜⊏(⋅1) ⊃M.Index(⋅(↯N 0))
Pack ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
~Frame {Ids Cy Iter Rbytes Ibytes Cx}
FrameOf ← Frame ⊃(M.Index|M.Cy|M.Iter|♭ bytes⌞ "f64" M.R|♭ bytes⌞ "f64" M.I|M.Cx)
~Recorder {State Frames}
ToAdvance ← Recorder ⊃(Advance Recorder.State|Recorder.Frames)
Capture ← Recorder ⊃(Prune Recorder.State|⊂⊃Recorder.Frames(□ FrameOf Recorder.State))
Logged ← Capture ToAdvance
~TracedRow {Packed Frames}
Row ← TracedRow ⊃(Pack Bits Recorder.State|Recorder.Frames) ⍢(Logged|×⊃(<50M.Iter Recorder.State)(>0⧻M.Index Recorder.State)) Recorder ⊃∘(⋅[]) M (↯N 0) (↯N 0) (⇡N) Cr ⊙0
T ← ↧8N
Groups ← ⊕□⊸(◿T)⇡N
Worker ← □ ≡Row -1 ×2÷N °□
Handles ← ≡spawnWorker Groups
Order ← /◇⊂ Groups
&p json ⊏ (⍏Order) /◇⊂ wait Handles
