⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Write S
 ⊣(S,⎕UCS 10)⎕FIO[23]OUT
∇
∇ WriteWrapped S;N;Lines;B
 N←+/⍴S
 Lines←⌊N÷60
 B←((Lines,60)⍴(60×Lines)↑S),(Lines,1)⍴⎕UCS 10
 ⊣(,B)⎕FIO[23]OUT
 →(0=60|N)/0
 ⊣(((60×Lines)↓S),⎕UCS 10)⎕FIO[23]OUT
∇
∇ Letters Generate Arg;CDF;N;Line;J;Len;R
 CDF←+\⊃Arg[0]
 N←⊃Arg[1]
L:→(N=0)/0
 Len←60000⌊N
 R←Len⍴0.0
 J←0
C:→(J≥Len)/Emit
 Seed←139968|29573+3877×Seed
 R[J]←Seed÷139968.0
 J←J+1
 →C
Emit:Line←Letters[+/[0]CDF∘.≤R]
 WriteWrapped Line
 N←N-Len
 →L
∇
∇ Run N;Alu;Offset;Len
 Alu←'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'
 Write '>ONE Homo sapiens alu'
 Offset←0
L:→(Offset≥2×N)/Random
 Len←60000⌊(2×N)-Offset
 WriteWrapped Alu[(⍴Alu)|Offset+⍳Len]
 Offset←Offset+Len
 →L
Random:Seed←42
 Write '>TWO IUB ambiguity codes'
 'acgtBDHKMNRSVWY' Generate (⊂0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02),⊂3×N
 Write '>THREE Homo sapiens frequency'
 'acgt' Generate (⊂0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008),⊂5×N
∇
Run ⍎,⊃¯1↑⎕ARG

⊣⎕FIO[4]OUT
)OFF
