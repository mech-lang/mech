// Faithful Benchmarks Game workload; local Toit candidate.
import io.stdio

main args:
  n := int.parse args[0]
  print "P4\n$n $n"
  row := ByteArray ((n + 7) / 8)
  for y := 0; y < n; y++:
    ci := 2.0 * y / n - 1.0
    bits := 0
    for x := 0; x < n; x++:
      cr := 2.0 * x / n - 1.5
      zr := 0.0
      zi := 0.0
      zr2 := 0.0
      zi2 := 0.0
      for iteration := 0; iteration < 50; iteration++:
        zi = 2.0 * zr * zi + ci
        zr = zr2 - zi2 + cr
        zr2 = zr * zr
        zi2 = zi * zi
        if zr2 + zi2 > 4.0: break
      bits = (bits << 1) | (zr2 + zi2 <= 4.0 ? 1 : 0)
      if (x & 7) == 7:
        row[x / 8] = bits
        bits = 0
    if (n & 7) != 0: row[row.size - 1] = bits << (8 - (n & 7))
    stdio.stdout.write row
