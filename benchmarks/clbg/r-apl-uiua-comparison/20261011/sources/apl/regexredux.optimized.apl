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
⍝ Replace any sequence of nonempty, nonoverlapping regex matches by one fixed string.
∇ Z←Pattern Replace Arg;S;Replacement;Matches;Starts;Lengths;Delta;Keep;Prefix;Width;Destinations
 S←⊃Arg[0]
 Replacement←,⊃Arg[1]
 Matches←Pattern⎕RE['g↓']S
 Z←S
 →(0=⍴Matches)/0
 Starts←∊{0⊃⍵}¨Matches
 Lengths←∊{1⊃⍵}¨Matches
 Delta←(1+⍴S)⍴0
 Delta[Starts]←Delta[Starts]+1
 Delta[Starts+Lengths]←Delta[Starts+Lengths]-1
 Keep←0=¯1↓+\Delta
 Width←+/⍴Replacement
 Delta←Keep
 Delta[Starts]←Delta[Starts]+Width
 Prefix←+\Delta
 Z←(+/Delta)⍴' '
 Z[(Keep/Prefix)-1]←Keep/S
 Destinations←Prefix[Starts]-Width
 Z[,Destinations∘.+⍳Width]←((⍴Starts)×Width)⍴Replacement
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
