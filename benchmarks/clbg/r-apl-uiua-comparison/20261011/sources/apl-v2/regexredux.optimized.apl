⍝ GNU APL 2.0; canonical orchestration with general PCRE2 global substitution.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'
∇ Z←Read;B
 Z←''
L:B←1048576⎕FIO[6]0
 →(0=⍴⍴B)/0
 Z←Z,⎕UCS B
 →L
∇
∇ Run;Input;S;Patterns;I;P;M;Initial;Cleaned
 Input←Read
 Initial←+/⍴Input
 S←('>[^\n]*\n|\n' '') Substitute[1] Input
 Input←''
 Cleaned←+/⍴S
 Patterns←'agggtaaa|tttaccct' 'a[act]ggtaaa|tttacc[agt]t' 'ag[act]gtaaa|tttac[agt]ct' 'agg[act]taaa|ttta[agt]cct' 'aggg[acg]aaa|ttt[cgt]ccct' 'agggt[cgt]aa|tt[acg]accct' 'agggta[cgt]a|t[acg]taccct' 'agggtaa[cgt]|[acg]ttaccct'
 Patterns←(⊂'agggtaaa|tttaccct'),(⊂'[cgt]gggtaaa|tttaccc[acg]'),1↓Patterns
 I←0
L:→(I≥⍴Patterns)/ReplaceAll
 P←⊃Patterns[I]
 M←P⎕RE['g']S
 ⊣((P,' %d',⎕UCS 10)(+/⍴M))⎕FIO[22]OUT
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
