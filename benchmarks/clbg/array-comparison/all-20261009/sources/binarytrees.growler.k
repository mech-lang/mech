\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
/ Every node is an allocated pair; checks traverse the constructed trees.
n:6|.`argv 2
N:{(x;y)}
T:{$[x;N[T x-1;T x-1];N[x#();x#()]]}
C:{$[0=#*x;1;1+/C'x]}
`0:"stretch tree of depth ",($(n+1)),"\t check: ",$C T n+1
long:T n
one:{C T x}
compute:{
 iterations:*/((n-x)+4)#2
 total:+/one'iterations#x
 (iterations;x;total)}
results:peach[compute;4+2*!1+_(n-4)%2]
report:{`0:($x 0),"\t trees of depth ",($x 1),"\t check: ",$x 2}
done:report'results
`0:"long lived tree of depth ",($n),"\t check: ",$C long
