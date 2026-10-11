⍝ GNU APL 2.0; streaming records and seven ordinary growing frequency maps.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
Bound←65536
Start←1
Mode←0
Selected←0
Header←''
∇ MapNew Dummy
 Keys←17⍴0
 Counts←17⍴0
 Used←0
∇
∇ Amount MapAdd Key;Slot
 →((10×Used)<7×⍴Keys)/Locate
 MapGrow
Locate:Slot←(⍴Keys)|Key
Probe:→(Counts[Slot]=0)/Insert
 →(Keys[Slot]=Key)/Increment
 Slot←(⍴Keys)|Slot+1
 →Probe
Insert:Keys[Slot]←Key
 Used←Used+1
Increment:Counts[Slot]←Counts[Slot]+Amount
∇
∇ MapGrow;OldKeys;OldCounts;Positions;Cap;I
 OldKeys←Keys
 OldCounts←Counts
 Positions←(OldCounts≠0)/⍳⍴OldCounts
 Cap←1+2×+/⍴Keys
 Keys←Cap⍴0
 Counts←Cap⍴0
 Used←0
 I←0
L:→(I≥⍴Positions)/0
 OldCounts[Positions[I]] MapAdd OldKeys[Positions[I]]
 I←I+1
 →L
∇
∇ Z←Key MapFind Map;TableKeys;TableCounts;Slot;Cap
 TableKeys←⊃Map[0]
 TableCounts←⊃Map[1]
 Cap←+/⍴TableKeys
 Slot←Cap|Key
 Z←0
L:→(TableCounts[Slot]=0)/0
 →(TableKeys[Slot]=Key)/Found
 Slot←Cap|Slot+1
 →L
Found:Z←TableCounts[Slot]
∇
∇ Init;I
 Sizes←1 2 3 4 6 12 18
 Maps←⍬
 I←0
L:→(I≥⍴Sizes)/End
 MapNew 0
 Maps←Maps,⊂((⊂Keys),(⊂Counts),⊂Used)
 I←I+1
 →L
End:Keys←⍬
 Counts←⍬
 Carries←(⍴Sizes)⍴⊂⍬
 Total←0
∇
∇ AddWindows Fresh;I;K;Joined;N;Map;J;Key;Limit
 Total←Total+(+/⍴Fresh)
 I←0
L:→(I≥⍴Sizes)/0
 K←Sizes[I]
 Joined←(⊃Carries[I]),Fresh
 N←0⌈1+(+/⍴Joined)-K
 →(N=0)/Keep
 Map←⊃Maps[I]
 Maps[I]←⊂⍬
 Keys←⊃Map[0]
 Counts←⊃Map[1]
 Used←⊃Map[2]
 Map←⍬
 J←0
 Key←4⊥K↑Joined
 Limit←4*K
Count:1 MapAdd Key
 J←J+1
 →(J≥N)/Save
 Key←(Limit|4×Key)+Joined[(J+K)-1]
 →Count
Save:Maps[I]←⊂((⊂Keys),(⊂Counts),⊂Used)
 Keys←⍬
 Counts←⍬
Keep:Carries[I]←⊂(-((K-1)⌊⍴Joined))↑Joined
 I←I+1
 →L
∇
∇ Feed Text
 →(Selected≠1)/0
 Text←(~Text∊⎕UCS 10 13)/Text
 →(0=⍴Text)/0
 Text←⎕UCS (⎕UCS Text)-32×Text∊'acgt'
 AddWindows 'ACGT'⍳Text
∇
∇ FinishHeader
 →(Selected≠0)/0
 Selected←Header≡'>THREE'
∇
∇ Parse Text;Count;Matches;I;Pos;At;Len;Part
 →(~Mode∊1 3)/Ready
 Count←(⍴Text)⌊1+Text⍳⎕UCS 10
 →(Mode≠1)/Skip
 Part←Count↑Text
 Header←(6⌊⍴(Header,Part))↑Header,Part
Skip:Start←(⎕UCS 10)=Text[Count-1]
 Text←Count↓Text
 →(~Start)/0
 →(Mode≠1)/Continued
 FinishHeader
Continued:Mode←0
 →(0=⍴Text)/0
Ready:Text←(⎕UCS 32-22×Start),Text
 Matches←'(?m)^[>;][^\n]*(?:\n|$)'⎕RE['g↓']Text
 I←0
 Pos←1
L:→(I≥⍴Matches)/Tail
 At←0⊃⊃Matches[I]
 Len←1⊃⊃Matches[I]
 Feed (At-Pos)↑Pos↓Text
 Part←Len↑At↓Text
 Mode←3
 →('>'≠Part[0])/Next
 →(Selected≠1)/HeaderStart
 Selected←2
HeaderStart:Header←(6⌊⍴Part)↑Part
 Mode←1
Next:→((⎕UCS 10)≠Part[¯1+⍴Part])/Advance
 →(Mode≠1)/Complete
 FinishHeader
Complete:Mode←0
Advance:Pos←At+Len
 I←I+1
 →L
Tail:Feed Pos↓Text
 Start←(⎕UCS 10)=Text[¯1+⍴Text]
∇
∇ Frequencies Arg;K;Map;Positions;KC;CC;Order;I;Text;Percentage
 K←⊃Arg[0]
 Map←⊃Arg[1]
 KC←⊃Map[0]
 CC←⊃Map[1]
 Positions←(CC≠0)/⍳⍴CC
 KC←KC[Positions]
 CC←CC[Positions]
 Order←⍋⍉(2,⍴KC)⍴(-CC),KC
 I←0
L:→(I≥⍴Order)/End
 Text←'ACGT'[(K⍴4)⊤KC[Order[I]]]
 Percentage←100.0×CC[Order[I]]÷1+Total-K
 ⊣(('%s %.3f',⎕UCS 10) Text Percentage)⎕FIO[22]OUT
 I←I+1
 →L
End:⊣(⎕UCS 10)⎕FIO[23]OUT
∇
∇ Display;Queries;I;Q;Key;Value
 Frequencies (⊂1),Maps[0]
 Frequencies (⊂2),Maps[1]
 Queries←'GGT' 'GGTA' 'GGTATT' 'GGTATTTTAATT' 'GGTATTTTAATTTATAGT'
 I←0
L:→(I≥⍴Queries)/0
 Q←⊃Queries[I]
 Key←4⊥'ACGT'⍳Q
 Value←Key MapFind ⊃Maps[I+2]
 ⊣(('%d',(⎕UCS 9),'%s',⎕UCS 10) Value Q)⎕FIO[22]OUT
 I←I+1
 →L
∇
∇ Run;B
 Init
L:B←Bound⎕FIO[6]0
 →(0=+/⍴B)/End
 Parse ⎕UCS B
 →L
End:Display
∇
∇ Width Dump Map;KC;CC;Positions;Order;I
 KC←⊃Map[0]
 CC←⊃Map[1]
 Positions←(CC≠0)/⍳⍴CC
 Order←Positions[⍋KC[Positions]]
 I←0
L:→(I≥⍴Order)/0
 ⊣(('%d %.0f %lld',⎕UCS 10) Width KC[Order[I]] CC[Order[I]])⎕FIO[22]OUT
 I←I+1
 →L
∇
∇ DumpAll;I
 I←0
L:→(I≥⍴Sizes)/0
 Sizes[I] Dump ⊃Maps[I]
 I←I+1
 →L
∇

MapNew 0
1 MapAdd 0
2 MapAdd 17
3 MapAdd 34
4 MapAdd 51
5 MapAdd 0
6 MapAdd ¯17
7 MapAdd ¯34
8 MapAdd 17
2 MapAdd 51
3 MapAdd 68
4 MapAdd ¯17
5 MapAdd 85
6 MapAdd 102
7 MapAdd 0
8 MapAdd 17
9 MapAdd 34
0 Dump ((⊂Keys),(⊂Counts),⊂Used)
⊣⎕FIO[4]OUT
)OFF
