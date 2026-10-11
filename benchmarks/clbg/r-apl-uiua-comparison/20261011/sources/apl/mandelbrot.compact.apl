⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;X;Y;CR;CI;ZR;ZI;R2;I2;Alive;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 X←⍳N
 CR←¯1.5+2.0×X÷N
 Pad←8×⌈N÷8
 Y←0
Row:→(Y≥N)/0
 CI←¯1+2.0×Y÷N
 ZR←N⍴0.0
 ZI←N⍴0.0
 Alive←N⍴1
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
 →(∨/Alive)/Loop
Emit:Bits←Pad↑Alive
 Bytes←2⊥⍉((Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Y←Y+1
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
