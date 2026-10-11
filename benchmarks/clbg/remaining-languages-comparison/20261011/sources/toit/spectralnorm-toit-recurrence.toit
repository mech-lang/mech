// Faithful Benchmarks Game workload; local Toit candidate.
import math

multiply source/List output/List transpose/bool:
  n := source.size
  for i := 0; i < n; i++:
    denominator := i * (i + 1) / 2 + (transpose ? 1 : i + 1)
    increment := i + (transpose ? 2 : 1)
    sum := 0.0
    for j := 0; j < n; j++:
      sum += (1.0 / denominator) * source[j]
      denominator += increment
      increment++
    output[i] = sum

main args:
  n := int.parse args[0]
  u := List n 1.0
  v := List n 0.0
  scratch := List n 0.0
  10.repeat:
    multiply u scratch false
    multiply scratch v true
    multiply v scratch false
    multiply scratch u true
  uv := 0.0
  vv := 0.0
  for i := 0; i < n; i++:
    uv += u[i] * v[i]
    vv += v[i] * v[i]
  print "$(%.9f (math.sqrt (uv / vv)))"
