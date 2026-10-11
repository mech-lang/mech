⍝ GNU APL 2.0; bounded bulk parsing and arrays, all sequence work remains APL.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
Scratch←,⊃¯1↑⎕ARG
File←¯1
Start←1
Mode←0
∇ Emit Handle;Remaining;Count;B;S;Lines;From;To
 Remaining←⎕FIO[12]Handle
 From←'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy'
 To←'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR'
L:→(Remaining=0)/End
 Count←245760⌊Remaining
 Remaining←Remaining-Count
 ⊣Remaining⎕FIO[13]Handle
 B←Count⎕FIO[6]Handle
 S←To[From⍳⌽⎕UCS B]
 Lines←⌊Count÷60
 ⊣(,((Lines,60)⍴(60×Lines)↑S),(Lines,1)⍴⎕UCS 10)⎕FIO[23]OUT
 →(0=60|Count)/L
 ⊣(((60×Lines)↓S),⎕UCS 10)⎕FIO[23]OUT
 →L
End:⊣⎕FIO[4]Handle
∇
∇ Store Text
 →(File<0)/0
 ⊣(⎕UCS(~Text∊⎕UCS 10 13)/Text)⎕FIO[7]File
∇
∇ Parse Text;Count;Matches;I;Pos;At;Len;Part
 →(~Mode∊1 3)/Ready
 Count←(⍴Text)⌊1+Text⍳⎕UCS 10
 →(Mode≠1)/Skip
 ⊣(((Count↑Text)≠⎕UCS 13)/Count↑Text)⎕FIO[23]OUT
Skip:Start←(⎕UCS 10)=Text[Count-1]
 Text←Count↓Text
 →(~Start)/0
 Mode←2×File≥0
 →(0=⍴Text)/0
Ready:Text←(⎕UCS 32-22×Start),Text
 Matches←'(?m)^[>;][^\n]*(?:\n|$)'⎕RE['g↓']Text
 I←0
 Pos←1
L:→(I≥⍴Matches)/Tail
 At←0⊃⊃Matches[I]
 Len←1⊃⊃Matches[I]
 Store (At-Pos)↑Pos↓Text
 Part←Len↑At↓Text
 Mode←3
 →('>'≠Part[0])/Next
 →(File<0)/New
 Emit File
New:File←'w+'⎕FIO[3]Scratch
 Mode←1
 ⊣((Part≠⎕UCS 13)/Part)⎕FIO[23]OUT
Next:→((⎕UCS 10)≠Part[¯1+⍴Part])/Advance
 Mode←2×File≥0
Advance:Pos←At+Len
 I←I+1
 →L
Tail:Store Pos↓Text
 Start←(⎕UCS 10)=Text[¯1+⍴Text]
∇
∇ Run;B
L:B←245760⎕FIO[6]0
 →(0=+/⍴B)/End
 Parse ⎕UCS B
 →L
End:→(File<0)/0
 →((Mode≠1)∨Start)/Flush
 ⊣(⎕UCS 10)⎕FIO[23]OUT
Flush:Emit File
∇
Run
⊣⎕FIO[4]OUT
)OFF
