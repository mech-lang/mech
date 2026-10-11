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
∇ Emit Arg;Header;S;I;From;To
 Header←⊃Arg[0]
 S←⌽⊃Arg[1]
 From←'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy'
 To←'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR'
 S←To[From⍳S]
 ⊣(Header,⎕UCS 10)⎕FIO[23]OUT
 I←0
L:→(I≥⍴S)/0
 ⊣(((60⌊(⍴S)-I)↑I↓S),⎕UCS 10)⎕FIO[23]OUT
 I←I+60
 →L
∇
∇ Run;Input;Lines;I;Line;Header;S
 Input←Read
 Lines←(Input≠⎕UCS 10)⊂Input
 I←0
 Header←''
 S←''
L:→(I≥⍴Lines)/End
 Line←⊃Lines[I]
 →(0=⍴Line)/Next
 →(';'=Line[0])/Next
 →('>'≠Line[0])/Sequence
 →(0=⍴Header)/HeaderLine
 Emit (⊂Header),⊂S
HeaderLine:Header←(Line≠⎕UCS 13)/Line
 S←''
 →Next
Sequence:S←S,(Line≠⎕UCS 13)/Line
Next:I←I+1
 →L
End:→(0=⍴Header)/0
 Emit (⊂Header),⊂S
∇
Run

⊣⎕FIO[4]OUT
)OFF
