N            ← ⋕◇∘⊡1 &args
Solar        ← ×4˙×π
Masses       ← ×Solar [1 0.0009547919384243266 0.0002858859806661308 0.00004366244043351563 0.000051513890204661145]
Positions    ← [0_0_0 4.84143144246472090_¯1.1603200440274284_¯0.103622044471123109 8.34336671824458_4.124798564124305_¯0.4035234171143214 12.8943695621391310_¯15.111151401698631_¯0.22330757889265573 15.379697114850917_¯25.919314609987964_0.17925877295037118]
Velocities   ← ×365.24 [0_0_0 0.001660076642744037_0.007699011184197404_¯0.0000690460016972063 ¯0.002767425107268624_0.004998528012349172_0.000023041729757376393 0.002964601375647616_0.00237847173959480950_¯0.000029658956854023756 0.0026806777249038932_0.001628241700382423_¯0.0000951592254519715870]
Initial      ← ≡⊂ Positions ⊂¤(¯÷Solar/+×⍉¤Masses Velocities) ↘1 Velocities
Pos          ← ≡↙3
Vel          ← ≡↘3
Differences  ← ˙⊞- Pos
Norm         ← ≡≡/+˙×
Diagonal     ← ˙⊞= ⇡5
Upper        ← ˙⊞> ⇡5
Acceleration ← ≡/+ ÷⊃(≡≡¤ √ⁿ3+Diagonal Norm)(×↯1_5_1 Masses) Differences
NewVelocity  ← +⊃Vel(×0.01 Acceleration)
Advance      ← ≡⊂ ⊃(+⊃Pos(×0.01 NewVelocity))NewVelocity
Potential    ← /+♭ ×Upper ÷⊃(√+Diagonal Norm Differences)(⋅(˙⊞×Masses))
Energy       ← -⊃Potential(×0.5/+×Masses ≡/+˙×Vel)
Fixed        ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9
Print        ← &p ⊂"-" Fixed ⌵ Energy
Print Initial
Print ⍥Advance N Initial
