⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Z←Read;B
 Z←''
L:B←1048576⎕FIO[6]0
 →(0=⍴⍴B)/0
 Z←Z,⎕UCS B
 →L
∇
∇ Z←Pattern Replace Arg;S;Replacement;Matches;Pos;Len;I;Last
 S←⊃Arg[0]
 Replacement←⊃Arg[1]
 Matches←Pattern⎕RE['g↓']S
 Z←''
 Last←0
 I←0
L:→(I≥⍴Matches)/End
 Pos←0⊃⊃Matches[I]
 Len←1⊃⊃Matches[I]
 Z←Z,((Pos-Last)↑Last↓S),Replacement
 Last←Pos+Len
 I←I+1
 →L
End:Z←Z,Last↓S
∇
∇ Run;Input;S;Patterns;I;P;M;Initial;Cleaned
 Input←Read
 Initial←+/⍴Input
 S←'>[^\n]*\n|\n' Replace (⊂Input),⊂''
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
ReplaceAll:S←'tHa[Nt]' Replace (⊂S),⊂'<4>'
 S←'aND|caN|Ha[DS]|WaS' Replace (⊂S),⊂'<3>'
 S←'a[NSt]|BY' Replace (⊂S),⊂'<2>'
 S←'<[^>]*>' Replace (⊂S),⊂'|'
 S←'\|[^|][^|]*\|' Replace (⊂S),⊂'-'
 ⊣(((⎕UCS 10),'%d',(⎕UCS 10),'%d',(⎕UCS 10),'%d',⎕UCS 10) Initial Cleaned (+/⍴S))⎕FIO[22]OUT
∇
Run

⊣⎕FIO[4]OUT
)OFF
