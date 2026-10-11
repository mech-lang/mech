⍝ GNU APL 2.0; explicit canonical algorithm, authored for this comparison.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libgmp-apl.dylib'⎕FX'BigInteger'
∇ Z←Extract J
 ⊣Scalar BigInteger[3]J
 ⊣Numerator BigInteger[6]Q Scalar
 ⊣Numerator BigInteger[4]Numerator R
 ⊣Numerator BigInteger[7]Numerator T
 Z←BigInteger[9]Numerator
∇
∇ Run N;K;Printed;Digit;Next
 Q←BigInteger[0]1
 R←BigInteger[0]0
 T←BigInteger[0]1
 Scalar←BigInteger[0]0
 Numerator←BigInteger[0]0
 Temporary←BigInteger[0]0
 K←1
 Printed←0
L:→(Printed≥N)/End
 Digit←Extract 3
 Next←Extract 4
 →(Digit≠Next)/Consume
 ⊣('%d' Digit)⎕FIO[22]OUT
 Printed←Printed+1
 →(0≠10|Printed)/Produce
 ⊣(((⎕UCS 9),':%d',⎕UCS 10) Printed)⎕FIO[22]OUT
Produce:⊣Scalar BigInteger[3]Digit
 ⊣Temporary BigInteger[6]T Scalar
 ⊣R BigInteger[5]R Temporary
 ⊣Scalar BigInteger[3]10
 ⊣R BigInteger[6]R Scalar
 ⊣Q BigInteger[6]Q Scalar
 →L
Consume:⊣Scalar BigInteger[3]2
 ⊣Temporary BigInteger[6]Q Scalar
 ⊣R BigInteger[4]R Temporary
 ⊣Scalar BigInteger[3]1+2×K
 ⊣R BigInteger[6]R Scalar
 ⊣T BigInteger[6]T Scalar
 ⊣Scalar BigInteger[3]K
 ⊣Q BigInteger[6]Q Scalar
 K←K+1
 →L
End:→(0=10|Printed)/Free
 ⊣((((10-10|Printed)⍴' '),(⎕UCS 9),':%d',⎕UCS 10) Printed)⎕FIO[22]OUT
Free:⊣BigInteger[1]Q
 ⊣BigInteger[1]R
 ⊣BigInteger[1]T
 ⊣BigInteger[1]Scalar
 ⊣BigInteger[1]Numerator
 ⊣BigInteger[1]Temporary
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
