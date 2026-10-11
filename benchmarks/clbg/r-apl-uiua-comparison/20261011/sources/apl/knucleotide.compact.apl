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
⍝ Generic growing open-addressed integer-key counter, with explicit collision equality.
∇ MapNew Dummy
 Capacity←17
 Keys←Capacity⍴0
 Counts←Capacity⍴0
 Used←0
∇
∇ Amount MapAdd Key;Slot
 →((10×Used)<7×Capacity)/Locate
 MapGrow
Locate:Slot←Capacity|Key
Probe:→(Counts[Slot]=0)/Insert
 →(Keys[Slot]=Key)/Increment
 Slot←Capacity|Slot+1
 →Probe
Insert:Keys[Slot]←Key
 Used←Used+1
Increment:Counts[Slot]←Counts[Slot]+Amount
∇
∇ MapGrow;OldKeys;OldCounts;Positions;I
 OldKeys←Keys
 OldCounts←Counts
 Positions←(Counts≠0)/⍳Capacity
 Capacity←1+2×Capacity
 Keys←Capacity⍴0
 Counts←Capacity⍴0
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
∇ Z←Count K;N;I;Key;Limit
 MapNew 0
 N←1+(⍴Codes)-K
 I←0
 Key←4⊥K↑Codes
 Limit←4*K
L:→(I≥N)/End
 1 MapAdd Key
 I←I+1
 →(I≥N)/End
 Key←(Limit|4×Key)+Codes[(I+K)-1]
 →L
End:Z←(⊂Keys),⊂Counts
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
 Percentage←100.0×CC[Order[I]]÷1+(+/⍴Codes)-K
 ⊣(('%s %.3f',⎕UCS 10) Text Percentage)⎕FIO[22]OUT
 I←I+1
 →L
End:⊣(⎕UCS 10)⎕FIO[23]OUT
∇
∇ Run;Input;Match;Start;S;Stops;Sizes;Maps;I;Map;Queries;Q;Key;Value
 Input←Read
 Match←'>THREE[^\n]*\n'⎕RE['g↓']Input
 S←''
 →(0=⍴Match)/Clean
 Start←+/⊃Match[0]
 S←Start↓Input
 Stops←'>[^\n]*'⎕RE['g↓']S
 →(0=⍴Stops)/Clean
 S←(0⊃⊃Stops[0])↑S
Clean:S←(~'(?m)^;[^\n]*'⎕RE['/g']S)/S
 S←(~S∊⎕UCS 10 13)/S
 S←⎕UCS (⎕UCS S)-32×S∊'acgt'
 Codes←'ACGT'⍳S
 Sizes←1 2 3 4 6 12 18
 Maps←⍬
 I←0
Build:→(I≥⍴Sizes)/Display
 Map←Count Sizes[I]
 Maps←Maps,⊂Map
 I←I+1
 →Build
Display:Frequencies (⊂1),Maps[0]
 Frequencies (⊂2),Maps[1]
 Queries←'GGT' 'GGTA' 'GGTATT' 'GGTATTTTAATT' 'GGTATTTTAATTTATAGT'
 I←0
Query:→(I≥⍴Queries)/0
 Q←⊃Queries[I]
 Key←4⊥'ACGT'⍳Q
 Value←Key MapFind ⊃Maps[I+2]
 ⊣(('%d',(⎕UCS 9),'%s',⎕UCS 10) Value Q)⎕FIO[22]OUT
 I←I+1
 →Query
∇
Run
⊣⎕FIO[4]OUT
)OFF
