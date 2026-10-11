Input    ← &fras "/dev/stdin"
Initial  ← ⧻Input
Clean    ← ⍜regex≡(⋅"") ">.*\n|\n"
Sequence ← Clean Input
Length   ← ⧻Sequence
Patterns ← {"agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]" "a[act]ggtaaa|tttacc[agt]t" "ag[act]gtaaa|tttac[agt]ct" "agg[act]taaa|ttta[agt]cct" "aggg[acg]aaa|ttt[cgt]ccct" "agggt[cgt]aa|tt[acg]accct" "agggta[cgt]a|t[acg]taccct" "agggtaa[cgt]|[acg]ttaccct"}
Count    ← &p $"_ _" ⟜(⧻regex ⊙Sequence) ◇∘
≡Count Patterns
Replacements  ← {"<4>" "<3>" "<2>" "|" "-"}
Substitutions ← {"tHa[Nt]" "aND|caN|Ha[DS]|WaS" "a[NSt]|BY" "<[^>]*>" "\\|[^|][^|]*\\|"}
Replace       ← ⍜regex(≡⌟⋅∘) ⊃(◇∘⊡0|⋅∘|◇∘⊡1)
Final         ← ∧Replace ≡⊟ Substitutions Replacements Sequence
&p ""
&p Initial
&p Length
&p ⧻Final
