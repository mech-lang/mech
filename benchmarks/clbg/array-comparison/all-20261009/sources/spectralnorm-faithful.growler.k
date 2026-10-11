/ Four functions; coefficients are computed for each matrix-vector multiply.
n:.`argv 2
A:{[i;j]1%1+i+0.5*(i+j)*1+i+j}
Av:{[v]{[v;i]+/v*A[i;!#v]}[v]'!#v}
Atv:{[v]{[v;i]+/v*A[!#v;i]}[v]'!#v}
AtAv:{Atv Av x}
v:19 AtAv/n#1.0
u:AtAv v
r:$_0.5+1e9*%(+/u*v)%+/v*v
`0:(-9_r),".",-9#r
