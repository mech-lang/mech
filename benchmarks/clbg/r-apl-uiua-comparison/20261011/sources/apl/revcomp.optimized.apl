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
∇ WriteWrapped S;N;Lines;B
 N←+/⍴S
 Lines←⌊N÷60
 B←((Lines,60)⍴(60×Lines)↑S),(Lines,1)⍴⎕UCS 10
 ⊣(,B)⎕FIO[23]OUT
 →(0=60|N)/0
 ⊣(((60×Lines)↓S),⎕UCS 10)⎕FIO[23]OUT
∇
∇ Run;Input;Matches;I;Pos;Len;Last;Next;Header;S;From;To
 Input←Read
 Matches←'(?m)^>[^\n]*(?:\n|$)'⎕RE['g↓']Input
 From←'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy'
 To←'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR'
 I←0
L:→(I≥⍴Matches)/0
 Pos←0⊃⊃Matches[I]
 Len←1⊃⊃Matches[I]
 Header←(Len-(Input[Pos+Len-1]=⎕UCS 10))↑Pos↓Input
 Header←(Header≠⎕UCS 13)/Header
 Last←+/⍴Input
 →((I+1)≥⍴Matches)/Body
 Last←0⊃⊃Matches[I+1]
Body:Next←Pos+Len
 S←(Last-Next)↑Next↓Input
 S←(~'(?m)^;[^\n]*'⎕RE['/g']S)/S
 S←(~S∊⎕UCS 10 13)/S
 S←To[From⍳⌽S]
 ⊣(Header,⎕UCS 10)⎕FIO[23]OUT
 WriteWrapped S
 I←I+1
 →L
∇
Run

⊣⎕FIO[4]OUT
)OFF
