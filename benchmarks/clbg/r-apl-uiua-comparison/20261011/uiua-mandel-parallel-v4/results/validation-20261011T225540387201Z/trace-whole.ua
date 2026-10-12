# Experimental!
N  ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Live Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Live|M.Cx|M.Cy|+1M.Iter)
Mark    ← M ⊃(M.R|M.I|×⊃M.Live(≤4+⊃(˙×M.R)(˙×M.I))|M.Cx|M.Cy|M.Iter)
Iterate ← Mark Advance
Pack    ← ≡/+ ×⊙(¤⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
~Frame {Ids Cy Iter Rbytes Ibytes Cx}
FrameOf ← Frame ⊃(⊚M.Live|M.Cy|M.Iter|♭ bytes⌞ "f64" ⊏⊃(⊚M.Live)M.R|♭ bytes⌞ "f64" ⊏⊃(⊚M.Live)M.I|⊏⊃(⊚M.Live)M.Cx)
~Recorder {State Frames}
ToAdvance ← Recorder ⊃(Advance Recorder.State|Recorder.Frames)
Capture ← Recorder ⊃(Mark Recorder.State|⊂⊃Recorder.Frames(□ FrameOf Recorder.State))
Logged ← Capture ToAdvance
~TracedRow {Packed Frames}
Row ← TracedRow ⊃(Pack M.Live Recorder.State|Recorder.Frames) ⍢(Logged|×⊃(<50M.Iter Recorder.State)(/↥M.Live Recorder.State)) Recorder ⊃∘(⋅[]) M (↯N 0) (↯N 0) (↯N 1) Cr ⊙0
T ← ↧8N
Groups ← ⊕□⊸(◿T)⇡N
Worker ← □ ≡Row -1 ×2÷N °□
Handles ← ≡spawnWorker Groups
Order ← /◇⊂ Groups
&p json ⊏ (⍏Order) /◇⊂ wait Handles
