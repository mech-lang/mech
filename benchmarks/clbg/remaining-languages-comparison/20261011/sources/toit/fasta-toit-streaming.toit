// Faithful Benchmarks Game workload; local Toit candidate.
import io.stdio
import bytes show Buffer

ALU ::= """
    GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG\
    GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA\
    CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT\
    ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA\
    GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG\
    AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC\
    AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA\
    """
seed := 42

emit alphabet/string probabilities/List? length/int:
  thresholds := []
  total := 0.0
  if probabilities:
    probabilities.do:
      total += it
      thresholds.add total
  row := ByteArray 61
  output := Buffer
  offset := 0
  while offset < length:
    width := min 60 (length - offset)
    for column := 0; column < width; column++:
      if probabilities:
        seed = (seed * 3877 + 29573) % 139968
        value := seed.to-float / 139968.0
        low := 0
        high := thresholds.size - 1
        while low < high:
          middle := (low + high) / 2
          if value < thresholds[middle]: high = middle
          else: low = middle + 1
        row[column] = alphabet[low]
      else:
        row[column] = alphabet[(offset + column) % alphabet.size]
    row[width] = '\n'
    output.write row 0 (width + 1)
    if output.size >= 65536:
      stdio.stdout.write output.bytes
      output.clear
    offset += width
  stdio.stdout.write output.bytes

main args:
  n := int.parse args[0]
  print ">ONE Homo sapiens alu"
  emit ALU null (2 * n)
  print ">TWO IUB ambiguity codes"
  emit "acgtBDHKMNRSVWY" [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02] (3 * n)
  print ">THREE Homo sapiens frequency"
  emit "acgt" [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008] (5 * n)
