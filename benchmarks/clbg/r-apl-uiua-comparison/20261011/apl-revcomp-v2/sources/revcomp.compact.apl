⍝ GNU APL 2.0; bounded standard-I/O storage, all sequence work remains APL.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
Scratch←,⊃¯1↑⎕ARG
∇ Emit File;Remaining;Count;B;S;Lines;From;To
 Remaining←⎕FIO[12]File
 From←'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy'
 To←'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR'
L:→(Remaining=0)/End
 Count←245760⌊Remaining
 Remaining←Remaining-Count
 ⊣Remaining⎕FIO[13]File
 B←Count⎕FIO[6]File
 S←To[From⍳⌽⎕UCS B]
 Lines←⌊Count÷60
 ⊣(,((Lines,60)⍴(60×Lines)↑S),(Lines,1)⍴⎕UCS 10)⎕FIO[23]OUT
 →(0=60|Count)/L
 ⊣(((60×Lines)↓S),⎕UCS 10)⎕FIO[23]OUT
 →L
End:⊣⎕FIO[4]File
∇
∇ Run;File;Start;Mode;B;Line
 File←¯1
 Start←1
 Mode←0
L:B←262144⎕FIO[8]0
 →(0=+/⍴B)/End
 Line←⎕UCS B
 →(~Start)/Part
 Mode←2×File≥0
 →(';'≠Line[0])/HeaderTest
 Mode←3
 →Part
HeaderTest:→('>'≠Line[0])/Part
 →(File<0)/New
 Emit File
New:File←'w+'⎕FIO[3]Scratch
 Mode←1
Part:Start←(⎕UCS 10)=Line[¯1+⍴Line]
 →(Mode≠1)/Sequence
 ⊣((Line≠⎕UCS 13)/Line)⎕FIO[23]OUT
 →L
Sequence:→(Mode≠2)/L
 ⊣(⎕UCS(~Line∊⎕UCS 10 13)/Line)⎕FIO[7]File
 →L
End:→(File<0)/0
 →((Mode≠1)∨Start)/Flush
 ⊣(⎕UCS 10)⎕FIO[23]OUT
Flush:Emit File
∇
Run
⊣⎕FIO[4]OUT
)OFF
