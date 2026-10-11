⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Z←T Multiply U;N;I;J;Rows;K;A;Offset
 N←+/⍴U
 J←⍳N
 Z←N⍴0.0
 Offset←0
L:→(Offset≥N)/0
 Rows←64⌊N-Offset
 I←Offset+⍳Rows
 K←I∘.+J
 A←1.0÷1+((K×K+1)÷2)+((T=0)×I)∘.+((T=1)×J)
 Z[I]←A+.×U
 Offset←Offset+Rows
 →L
∇
Step←{1 Multiply 0 Multiply ⍵}
∇ Run N;U;V;K
 U←N⍴1.0
 K←0
L:→(K≥10)/E
 V←Step U
 U←Step V
 K←K+1
 →L
E:⊣(('%.9f',⎕UCS 10) (((+/U×V)÷+/V×V)*0.5))⎕FIO[22]OUT
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
