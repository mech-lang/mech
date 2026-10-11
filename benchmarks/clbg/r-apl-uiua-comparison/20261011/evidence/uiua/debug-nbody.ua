N ← ⋕◇⊡1 &args
Solar ← ×4˙×π
Masses ← ×Solar [1 0.000954791938424326609 0.000285885980666130812 0.0000436624404335156298 0.0000515138902046611451]
Positions ← [0_0_0 4.84143144246472090_¯1.16032004402742839_¯0.103622044471123139 8.34336671824457987_4.12479856412430479_¯0.403523417114321381 12.8943695621391310_¯15.1111514016986312_¯0.223307578892655734 15.3796971148509165_¯25.9193146099879641_0.179258772950371181]
Velocities ← ×365.24 [0_0_0 0.00166007664274403694_0.00769901118419740425_¯0.0000690460016972063023 ¯0.00276742510726862411_0.00499852801234917238_0.0000230417297573763929 0.00296460137564761618_0.00237847173959480950_¯0.0000296589568540237556 0.00268067772490389322_0.00162824170038242295_¯0.0000951592254519715870]
Initial ← ≡⊂ Positions ⊂¤(¯÷Solar/+×⍉¤Masses Velocities) ↘1 Velocities
Pos ← ≡↙3
Vel ← ≡↘3
Differences ← ˙⊞- Pos
Norm ← ≡≡/+˙×
Diagonal ← ˙⊞= ⇡5
Upper ← ˙⊞> ⇡5
Acceleration ← ≡/+ ÷⊃(≡≡(¤√˜ⁿ3)+Diagonal Norm)(×↯1_5_1 Masses) Differences
NewVelocity ← +⊃Vel(×0.01 Acceleration)
Advance ← ≡⊂ ⊃(+⊃Pos(×0.01 NewVelocity))NewVelocity
Potential ← /+♭ ×Upper ÷⊃(√+Diagonal Norm Differences)(⋅˙⊞×Masses)
Energy ← -⊃Potential(×0.5/+×Masses ≡/+˙×Vel)

&p Initial
&p Norm Differences Initial
&p Energy Initial
&p Potential Initial
&p ×0.5/+×Masses ≡/+˙×Vel Initial
&p ˙⊞×Masses
&p √+Diagonal Norm Differences Initial
&p ÷⊃(√+Diagonal Norm Differences)(⋅˙⊞×Masses) Initial
&p ×Upper ÷⊃(√+Diagonal Norm Differences)(⋅˙⊞×Masses) Initial
