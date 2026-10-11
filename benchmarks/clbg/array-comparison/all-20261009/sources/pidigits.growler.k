\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
/ Same sequential LFT spigot, using only generic GMP arithmetic.
n:.`argv 2
q:znew 1
r:znew 0
t:znew 1
a:znew 0
b:znew 0
k:0
digits:""
extract:{
 zmul[a;q;x]
 zadd[a;a;r]
 zdiv[b;a;t]
 zui b}
emit:{[d]
 digits::digits,"0123456789" d
 zmul[q;q;10]
 zmul[a;t;d]
 zsub[r;r;a]
 zmul[r;r;10]}
step:{
 k::k+1
 zmul[a;q;2]
 zadd[r;r;a]
 zmul[r;r;1+2*k]
 zmul[t;t;1+2*k]
 zmul[q;q;k]
 $[zcmp[q;r]>0;();{[d]$[d=extract 4;emit d;()]}extract 3]
 x}
done: {n>#digits}step/0
report:{[i]
 length:n&10+i
 line:(length-i)#i_digits
 `0:line,((10-#line)#" "),"\t:",$length}
done:report'10*!-_(-n)%10
