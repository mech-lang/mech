// Faithful Benchmarks Game workload; local Toit candidate.
import io.stdio
import bytes show Buffer

read-three -> ByteArray:
  found := false
  sequence := Buffer
  while line := stdio.stdin.read-line:
    if line.starts-with ">":
      if found: break
      found = line.starts-with ">THREE"
    else if found and not (line.starts-with ";"):
      sequence.write line.to-ascii-upper
  return sequence.bytes

encode-base c/int -> int:
  if c == 'A': return 0
  if c == 'C': return 1
  if c == 'G': return 2
  if c == 'T': return 3
  throw "INVALID_NUCLEOTIDE"

count-windows sequence/ByteArray k/int -> Map:
  counts := {:}
  mask := (1 << (2 * k)) - 1
  key := 0
  for i := 0; i < sequence.size; i++:
    key = ((key << 2) | (encode-base sequence[i])) & mask
    if i >= k - 1: counts[key] = (counts.get key --if-absent=: 0) + 1
  return counts

decode-key key/int k/int -> string:
  text := ByteArray k
  alphabet := "ACGT"
  for i := k - 1; i >= 0; i--:
    text[i] = alphabet[key & 3]
    key >>= 2
  return text.to-string

main args:
  sequence := read-three
  tables := {:}
  [1, 2, 3, 4, 6, 12, 18].do: tables[it] = count-windows sequence it
  [1, 2].do: | k |
    rows := []
    tables[k].do: | key value | rows.add [key, value]
    rows.sort --in-place: | a b | a[1] == b[1] ? (a[0] < b[0] ? -1 : 1) : (a[1] > b[1] ? -1 : 1)
    rows.do: print "$(decode-key it[0] k) $(%.3f (100.0 * it[1] / (sequence.size - k + 1)))"
    print
  ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"].do: | query |
    key := 0
    query.do: key = (key << 2) | (encode-base it)
    count := tables[query.size].get key --if-absent=: 0
    print "$count\t$query"
