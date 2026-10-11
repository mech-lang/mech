⍝ Canonical counted APL orchestration; ordinary PCRE2 JIT requested generally.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'
⊣'.build/apl-support/libregex-count-v3-apl.dylib'⎕FX'Count'
∇ Z←Read;B
 Z←''
L:B←1048576⎕FIO[6]0
 →(0=+/⍴B)/0
 Z←Z,⎕UCS B
 →L
∇
∇ Run;Input;S;Patterns;I;P;Initial;Cleaned
 Input←Read
 Initial←+/⍴Input
 S←('>[^\n]*\n|\n' '') Substitute[1] Input
 Input←''
 Cleaned←+/⍴S
 Patterns←'agggtaaa|tttaccct' '[cgt]gggtaaa|tttaccc[acg]' 'a[act]ggtaaa|tttacc[agt]t' 'ag[act]gtaaa|tttac[agt]ct' 'agg[act]taaa|ttta[agt]cct' 'aggg[acg]aaa|ttt[cgt]ccct' 'agggt[cgt]aa|tt[acg]accct' 'agggta[cgt]a|t[acg]taccct' 'agggtaa[cgt]|[acg]ttaccct'
 I←0
L:→(I≥⍴Patterns)/ReplaceAll
 P←⊃Patterns[I]
 ⊣((P,' %d',⎕UCS 10)(P Count[1] S))⎕FIO[22]OUT
 I←I+1
 →L
ReplaceAll:S←('tHa[Nt]' '<4>') Substitute[1] S
 S←('aND|caN|Ha[DS]|WaS' '<3>') Substitute[1] S
 S←('a[NSt]|BY' '<2>') Substitute[1] S
 S←('<[^>]*>' '|') Substitute[1] S
 S←('\|[^|][^|]*\|' '-') Substitute[1] S
 ⊣(((⎕UCS 10),'%d',(⎕UCS 10),'%d',(⎕UCS 10),'%d',⎕UCS 10) Initial Cleaned (+/⍴S))⎕FIO[22]OUT
∇
Run
⊣⎕FIO[4]OUT
)OFF
