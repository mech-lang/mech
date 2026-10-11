
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

      Naive transliteration from Sebastien Loisel's C program
      contributed by Isaac Gouy
*/      

import math

eval_A i/int j/int -> float:
  return 1.0/((i+j)*(i+j+1)/2+i+1)

eval_A_times_u N/int u/List Au/List:
  for i := 0; i < N; ++i:
    Au[i] = 0.0
    for j := 0; j < N; ++j:
      Au[i] += (eval_A i j) * u[j]      

eval_At_times_u N/int u/List Au/List:
  for i := 0; i < N; ++i:
    Au[i] = 0.0
    for j := 0; j < N; ++j:
      Au[i] += (eval_A j i) * u[j]
    
eval_AtA_times_u N/int u/List AtAu/List:
  v := List N; eval_A_times_u N u v; eval_At_times_u N v AtAu  

main args:
  N := args.size == 1 ? int.parse args[0] : 100
  u := List N 1.0; v := List N; vBv := 0.0; vv := 0.0
  for i := 0; i < 10; ++i:
    eval_AtA_times_u N u v
    eval_AtA_times_u N v u
  for i := 0; i < N; ++i: vBv += u[i] * v[i]; vv += v[i] * v[i]  
  print "$(%.9f (math.sqrt vBv / vv))" 
  
    