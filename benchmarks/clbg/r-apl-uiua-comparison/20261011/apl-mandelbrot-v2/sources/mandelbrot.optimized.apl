⍝ Counted APL: sixteen rows with stable compression of live pixel indices.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;Y;Offset;Rows;CR;CI;ZR;ZI;R2;I2;Alive;Index;Keep;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 Pad←8×⌈N÷8
 Offset←0
Row:→(Offset≥N)/0
 Rows←16⌊N-Offset
 Y←Offset+⍳Rows
 CR←(Rows×N)⍴¯1.5+2.0×(⍳N)÷N
 CI←,(¯1+2.0×Y÷N)∘.+N⍴0.0
 ZR←(Rows×N)⍴0.0
 ZI←(Rows×N)⍴0.0
 Alive←(Rows×N)⍴1
 Index←⍳Rows×N
 Iter←0
Loop:→(Iter≥50)/Emit
 R2←ZR×ZR
 I2←ZI×ZI
 ZI←CI[Index]+2×ZR×ZI
 ZR←CR[Index]+R2-I2
 Keep←4≥(ZR×ZR)+ZI×ZI
 Alive[Index]←Keep
 Index←Keep/Index
 ZR←Keep/ZR
 ZI←Keep/ZI
 Iter←Iter+1
 →(0<⍴Index)/Loop
Emit:Bits←(Rows,Pad)↑(Rows,N)⍴Alive
 Bytes←,⍉2⊥⍉(Rows,(Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Offset←Offset+Rows
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
