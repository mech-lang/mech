⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Z←Make D;Left;Right
 →(D>0)/Branch
 Z←0(⊂⍬)(⊂⍬)
 →0
Branch:Left←Make D-1
 Right←Make D-1
 Z←1 Left Right
∇
∇ Z←Check T
 Z←1
 →(0=⊃T[0])/0
 Z←1+(Check ⊃T[1])+Check ⊃T[2]
∇
∇ Run N;Maximum;Stretch;Long;D;Iterations;I;Sum;Temporary
 Maximum←6⌈N
 Stretch←Make Maximum+1
 ⊣(('stretch tree of depth %d',(⎕UCS 9),' check: %d',⎕UCS 10)(Maximum+1)(Check Stretch))⎕FIO[22]OUT
 Stretch←⍬
 Long←Make Maximum
 D←4
Depth:→(D>Maximum)/End
 Iterations←2*((Maximum-D)+4)
 I←0
 Sum←0
Loop:→(I≥Iterations)/Emit
 Temporary←Make D
 Sum←Sum+Check Temporary
 I←I+1
 →Loop
Emit:⊣(('%d',(⎕UCS 9),' trees of depth %d',(⎕UCS 9),' check: %d',⎕UCS 10) Iterations D Sum)⎕FIO[22]OUT
 D←D+2
 →Depth
End:⊣(('long lived tree of depth %d',(⎕UCS 9),' check: %d',⎕UCS 10) Maximum (Check Long))⎕FIO[22]OUT
∇
Run ⍎,⊃¯1↑⎕ARG

⊣⎕FIO[4]OUT
)OFF
