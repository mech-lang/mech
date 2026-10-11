# Nine independent scans and five substitutions in the required order.
std ← •Import "../stdlib/standard.bqn"
input ← •FChars "/dev/stdin"
initial ← ≠input
sequence ← std.RxReplace ⟨std.RxNew ">.*\n|\n",input,""⟩
cleaned ← ≠sequence
patterns ← ⟨"agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]","a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct","agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct","agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct","agggtaa[cgt]|[acg]ttaccct"⟩
Count ← {•Out 𝕩∾" "∾•Repr (std.RxNew 𝕩) std.RxCount sequence}
Count¨ patterns
Substitute ← {p‿r: sequence ↩ std.RxReplace ⟨std.RxNew p,sequence,r⟩ ⋄ @}
Substitute¨ ⟨"tHa[Nt]"‿"<4>","aND|caN|Ha[DS]|WaS"‿"<3>","a[NSt]|BY"‿"<2>","<[^>]*>"‿"|","\|[^|][^|]*\|"‿"-"⟩
•Out ""
•Out •Repr initial
•Out •Repr cleaned
•Out •Repr ≠sequence
