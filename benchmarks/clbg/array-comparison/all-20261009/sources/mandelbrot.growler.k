/ CLBG: full double-precision escape iteration and PBM byte output.
n:.`argv 2
cr:-1.5+2*(!n)%n
step:{[ci;z]
 rr:((z 0)*(z 0))-(z 1)*z 1
 zi:ci+2*(z 0)*z 1
 zr:cr+rr
 (zr;zi;(z 2)&~4<(zr*zr)+zi*zi)}
row:{[ci]
 z:50 step[ci]/(n#0.0;n#0.0;n#1)
 bits:(z 2),((8*-_(-n)%8)-n)#0
 bytes:{+/x*128 64 32 16 8 4 2 1}'0N 8#bits
 `1:`c$bytes}
`0:"P4"
`0:($n)," ",$n
done:row'(-1+2*(!n)%n)
