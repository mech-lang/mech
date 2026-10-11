// Faithful Benchmarks Game workload; local Toit candidate.
import bignum show Bignum

small-integer value/Bignum -> int:
  result := 0
  value.limbs_.do: result = (result << 8) | it
  return value.negative_ ? -result : result

main args:
  n := int.parse args[0]
  q := Bignum.hex "01"
  r := Bignum.hex "00"
  t := Bignum.hex "01"
  k := 0
  emitted := 0
  line := ""
  while emitted < n:
    k++
    factor := 2 * k + 1
    r = (r + q * 2) * factor
    q = q * k
    t = t * factor
    if (r - q).negative_: continue
    digit := (q * 3 + r) / t
    if digit != (q * 4 + r) / t: continue
    value := (small-integer digit)
    line += "$value"
    emitted++
    if emitted % 10 == 0 or emitted == n:
      print "$line$(" " * (10 - line.size))\t:$emitted"
      line = ""
    r = (r - t * digit) * 10
    q = q * 10
