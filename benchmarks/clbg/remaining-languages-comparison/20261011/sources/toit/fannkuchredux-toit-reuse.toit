// Faithful Benchmarks Game workload; local Toit candidate.
main args:
  n := int.parse args[0]
  permutation := List n: it
  scratch := List n 0
  counts := List n 0
  index := 0
  checksum := 0
  maximum := 0
  while true:
    scratch.replace 0 permutation
    flips := 0
    while scratch[0] != 0:
      end := scratch[0]
      for i := 0; 2 * i < end; i++:
        saved := scratch[i]
        scratch[i] = scratch[end - i]
        scratch[end - i] = saved
      flips++
    checksum += (index & 1) == 0 ? flips : -flips
    maximum = max maximum flips
    index++
    prefix := 2
    while prefix <= n:
      saved := permutation[0]
      for i := 0; i < prefix - 1; i++: permutation[i] = permutation[i + 1]
      permutation[prefix - 1] = saved
      counts[prefix - 1] = (counts[prefix - 1] + 1) % prefix
      if counts[prefix - 1] != 0: break
      prefix++
    if prefix > n: break
  print checksum
  print "Pfannkuchen($n) = $maximum"
