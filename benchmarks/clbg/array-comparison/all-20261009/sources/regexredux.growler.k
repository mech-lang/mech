\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
/ Nine scans remain independent; substitutions run sequentially.
input:1:0
initial:#input
sequence:rxreplace[rxnew ">.*\n|\n";input;""]
cleaned:#sequence
patterns:("agggtaaa|tttaccct";"[cgt]gggtaaa|tttaccc[acg]";"a[act]ggtaaa|tttacc[agt]t";"ag[act]gtaaa|tttac[agt]ct";"agg[act]taaa|ttta[agt]cct";"aggg[acg]aaa|ttt[cgt]ccct";"agggt[cgt]aa|tt[acg]accct";"agggta[cgt]a|t[acg]taccct";"agggtaa[cgt]|[acg]ttaccct")
count:{`0:x," ",$rxcount[rxnew x;sequence]}
done:count'patterns
substitute:{[p;r]sequence::rxreplace[rxnew p;sequence;r]}
done:substitute'[ ("tHa[Nt]";"aND|caN|Ha[DS]|WaS";"a[NSt]|BY";"<[^>]*>";"\\|[^|][^|]*\\|");("<4>";"<3>";"<2>";"|";"-")]
`0:""
`0:$initial
`0:$cleaned
`0:$#sequence
