Input    ← &fras "/dev/stdin"
Extract  ← ⨬(⋅""|◇∘⊡0_1) ⊸(>0⧻)
Clean    ← ⍜regex≡(⋅"") "(?m)^;[^\n]*\n|\r|\n"
Sequence ← ⊏⊙"ACGT" ◿4⊗"ACGTacgt" Clean Extract regex "(?m)^>THREE[^\n]*\n([^>]*)" Input
AddCount ← insert ⊃(∘|+1⬚0get|⋅∘)
~K {Width Start Counts}
WindowCount ← ↥0 +1 -⊃K.Width(⋅(⧻Sequence))
Remaining ← -⊃K.Start WindowCount
ChunkSize ← ↧65536 Remaining
ChunkText ← ↙⊃(+⊃ChunkSize(-1K.Width))(↘⊃K.Start(⋅Sequence))
ChunkWindows ← ⧈∘ ⊃K.Width ChunkText
ChunkCounts ← ∧AddCount ⊃ChunkWindows K.Counts
Advance ← K ⊃(K.Width|+⊃K.Start ChunkSize|ChunkCounts)
Count ← K.Counts ⍢(Advance|>0Remaining) K ⊙0 ⊙(map [] [])
Maps     ← ≡(□Count) [1 2 3 4 6 12 18]
Fixed    ← /◇⊂ {⊃(↘¯3|⋅"."|↙¯3)} ⬚@0↙¯⊸(↥4⧻) °⋕ ⁅×1e3
Entry    ← &p $"_ _" ⊃(◇∘⊡1)(Fixed ×100÷⊃(⋅∘)(¯◇∘⊡0))
Frequencies ← (
  ⊙⊸/+ °map
  ⊃(¯⋅⋅∘|∘|⋅∘)
  ≡⌟Entry ⍆≡{⊙∘}
  &p ""
)
Frequencies ◇∘⊡0 Maps
Frequencies ◇∘⊡1 Maps
Queries ← {"GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"}
≡(&p $"_\t_" ⊃(⬚0get ∩(◇∘))(◇∘)) Queries ↘2 Maps
