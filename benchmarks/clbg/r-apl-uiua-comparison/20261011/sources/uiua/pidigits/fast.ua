# Experimental!
N     ← ⋕◇∘⊡1 &args
Lib   ← &ffi ⊂□"/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/r-apl-uiua-comparison/20261011/.build/uiua-support/libgmp-uiua.dylib"
New   ← Lib {"void*" "ui_big_new" "long"} {∘}
Free  ← ◌ Lib {"void" "ui_big_free" "void*"} {∘}
Small ← Lib {"void*" "ui_big_mul_small" "void*" "void*" "long"}
Add   ← Lib {"void*" "ui_big_add" "void*" "void*" "void*"}
Sub   ← Lib {"void*" "ui_big_sub" "void*" "void*" "void*"}
Mul   ← Lib {"void*" "ui_big_mul" "void*" "void*" "void*"}
Div   ← Lib {"void*" "ui_big_floor_div" "void*" "void*" "void*"}
Get   ← Lib {"long" "ui_big_get" "void*"} {∘}
~P {Q R T A B D K Left Digits Written}
Triple         ← Small {⊃(P.B|P.Q|⋅3)}
Quadruple      ← Small {⊃(P.B|P.Q|⋅4)}
NumeratorThree ← Add {⊃(P.A|Triple|P.R)}
NumeratorFour  ← Add {⊃(P.A|Quadruple|P.R)}
ExtractThree   ← Get Div {⊃(P.D|NumeratorThree|P.T)}
ExtractFour    ← Get Div {⊃(P.D|NumeratorFour|P.T)}
~C {State Values}
~Attempt {State Digit Accept}
Candidates ← C ⊃(∘|⊃[ExtractThree|ExtractFour])
Decision   ← Attempt ⊃(C.State|⊢C.Values|/=C.Values) Candidates
GrowR      ← Small {⊃(P.R|Add {⊃(P.A|Small {⊃(P.B|P.Q|⋅2)}|P.R)}|+1×2P.K)}
GrowQ      ← Small {⊃(P.Q|P.Q|P.K)}
GrowT      ← Small {⊃(P.T|P.T|+1×2P.K)}
Consume ← (
  Attempt.State
  ⟜(◌GrowR)
  ⟜(◌GrowQ)
  ⟜(◌GrowT)
  P ⊃(P.Q|P.R|P.T|P.A|P.B|P.D|+1P.K|P.Left|P.Digits|P.Written)
)
EmitR       ← Small {⊃(P.R Attempt.State|Sub {⊃(P.A Attempt.State|P.R Attempt.State|Small {⊃(P.B Attempt.State|P.T Attempt.State|Attempt.Digit)})}|⋅10)}
EmitQ       ← Small {⊃(P.Q|P.Q|⋅10)}
EmitDigits  ← ⊂⊃(P.Digits Attempt.State)(°⋕Attempt.Digit)
PrintDigits ← &p $"_\t:_" ⊃(⬚@ ↙10 P.Digits|P.Written)
Flush       ← P ⊃(P.Q|P.R|P.T|P.A|P.B|P.D|P.K|P.Left|⋅""|P.Written) ⟜(PrintDigits)
MaybeFlush  ← ⨬(∘|Flush) ⊸(∨⊃(=0◿10P.Written)(=0P.Left))
Emit ← (
  ⟜(◌EmitR)
  ⟜(◌EmitQ Attempt.State)
  MaybeFlush P ⊃(P.Q Attempt.State|P.R Attempt.State|P.T Attempt.State|P.A Attempt.State|P.B Attempt.State|P.D Attempt.State|P.K Attempt.State|-1P.Left Attempt.State|EmitDigits|+1P.Written Attempt.State)
)
Step    ← ⨬(Consume|Emit) ⊸Attempt.Accept Decision
Initial ← P (New 1) (New 0) (New 1) (New 0) (New 0) (New 0) 1 N "" 0
Result  ← ⍢(Step|>0P.Left) Initial
≡(Free◇∘) {⊃(P.Q|P.R|P.T|P.A|P.B|P.D)} Result
