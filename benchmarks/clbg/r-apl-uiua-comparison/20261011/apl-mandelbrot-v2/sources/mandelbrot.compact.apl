⍝ Counted APL: compact row with stable compression of live pixel indices.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;Y;CR;CI;ZR;ZI;R2;I2;Alive;Index;Keep;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 CR←¯1.5+2.0×(⍳N)÷N
 Pad←8×⌈N÷8
 Y←0
Row:→(Y≥N)/0
 CI←¯1+2.0×Y÷N
 ZR←N⍴0.0
 ZI←N⍴0.0
 Alive←N⍴1
 Index←⍳N
 Iter←0
Loop:→(Iter≥50)/Emit
 R2←ZR×ZR
 I2←ZI×ZI
 ZI←CI+2×ZR×ZI
 ZR←CR[Index]+R2-I2
 Keep←4≥(ZR×ZR)+ZI×ZI
 Alive[Index]←Keep
 Index←Keep/Index
 ZR←Keep/ZR
 ZI←Keep/ZI
 Iter←Iter+1
 →(0<⍴Index)/Loop
Emit:Bits←Pad↑Alive
 Bytes←2⊥⍉((Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Y←Y+1
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
