⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;P;Copy;Count;R;F;K;Max;Sum;Index;First
 P←⍳N
 Count←N⍴0
 R←N
 Max←0
 Sum←0
 Index←0
Init:→(R≤1)/FlipStart
 Count[R-1]←R
 R←R-1
 →Init
FlipStart:Copy←P
 F←0
Flip:K←Copy[0]
 →(K=0)/Accumulate
 Copy[⍳K+1]←⌽Copy[⍳K+1]
 F←F+1
 →Flip
Accumulate:Max←Max⌈F
 Sum←Sum+F×1-2×2|Index
Rotate:→(R=N)/End
 First←P[0]
 P[⍳R]←P[1+⍳R]
 P[R]←First
 Count[R]←Count[R]-1
 →(Count[R]>0)/Next
 R←R+1
 →Rotate
Next:Index←Index+1
 →Init
End:⊣(('%d',(⎕UCS 10),'Pfannkuchen(%d) = %d',⎕UCS 10) Sum N Max)⎕FIO[22]OUT
∇
Run ⍎,⊃¯1↑⎕ARG

⊣⎕FIO[4]OUT
)OFF
