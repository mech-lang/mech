⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;Y;Offset;Rows;CR;CI;ZR;ZI;R2;I2;Alive;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 Pad←8×⌈N÷8
 Offset←0
Row:→(Offset≥N)/0
 Rows←16⌊N-Offset
 Y←Offset+⍳Rows
 CR←(Rows,N)⍴¯1.5+2.0×(⍳N)÷N
 CI←(¯1+2.0×Y÷N)∘.+N⍴0.0
 ZR←(Rows,N)⍴0.0
 ZI←(Rows,N)⍴0.0
 Alive←(Rows,N)⍴1
 Iter←0
Loop:→(Iter≥50)/Emit
 R2←ZR×ZR
 I2←ZI×ZI
 ZI←CI+2×ZR×ZI
 ZR←CR+R2-I2
 Alive←Alive∧4≥(ZR×ZR)+ZI×ZI
 ZR←ZR×Alive
 ZI←ZI×Alive
 Iter←Iter+1
 →(∨/,Alive)/Loop
Emit:Bits←(Rows,Pad)↑Alive
 Bytes←,⍉2⊥⍉(Rows,(Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Offset←Offset+Rows
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
