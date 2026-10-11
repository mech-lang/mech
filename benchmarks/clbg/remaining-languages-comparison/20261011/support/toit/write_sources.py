"""All local candidates are inspectable Toit applications; no host kernels."""
from pathlib import Path
import json

D = Path(__file__).resolve().parents[2]
S = D / 'sources/toit'
S.mkdir(parents=True, exist_ok=True)
rows = []


def app(benchmark, suffix, source, adaptations, upstream=None):
    candidate = f'{benchmark}-toit-{suffix}'
    path = S / (candidate+'.toit')
    path.write_text('// Faithful Benchmarks Game workload; local Toit candidate.\n'+source.strip()+'\n')
    row = dict(id=candidate, benchmark=benchmark, source=str(path),
               source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+benchmark+'.html',
               adaptations=adaptations, local_adaptation=True)
    if upstream:
        import hashlib
        row['upstream_source'] = str(D/'upstream/toit'/upstream)
        row['upstream_source_sha256'] = hashlib.sha256(Path(row['upstream_source']).read_bytes()).hexdigest()
    rows.append(row)


app('mandelbrot', 'rows', r'''
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
''', ['Canonical 50-iteration binary64 Mandelbrot from z=0, complete PBM including arbitrary-width row padding; reuse a row buffer and cached squares instead of one native write per byte.'], 'mandelbrot-toit-8.toit')


app('spectralnorm', 'recurrence', r'''
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
''', ['Use exact integer denominator recurrence and one scratch List for the same 40 full dense matrix/vector products; retain binary64 division, products and summation order.'], 'spectralnorm-toit-8.toit')


app('fannkuchredux', 'reuse', r'''
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
''', ['Generate every canonical prefix-rotation permutation with alternating checksum, including n=1; use standard List.replace into reusable scratch storage.'], 'fannkuchredux-toit-8.toit')


nbody = (D/'upstream/toit/nbody-toit-8.toit').read_text()
start = nbody.index('advance bodies/List dt/float:')
end = nbody.index('\nnbody n/int:', start)
nbody = nbody[:start] + '''advance bodies/List dt/float:
  n := bodies.size
  for i := 0; i < n; i++:
    a := bodies[i]
    for j := i + 1; j < n; j++:
      b := bodies[j]
      dx := a.x - b.x
      dy := a.y - b.y
      dz := a.z - b.z
      squared := dx * dx + dy * dy + dz * dz
      mag := dt / (squared * (math.sqrt squared))
      bm := b.mass * mag
      a.vx -= dx * bm
      a.vy -= dy * bm
      a.vz -= dz * bm
      am := a.mass * mag
      b.vx += dx * am
      b.vy += dy * am
      b.vz += dz * am
  for i := 0; i < n; i++:
    a := bodies[i]
    a.x += a.vx * dt
    a.y += a.vy * dt
    a.z += a.vz * dt
''' + nbody[end:]
app('nbody', 'cached-bodies', nbody, ['Cache each pair of Body references and use ordinary counted loops in the same five-body, all-pairs binary64 update; retain published solar constants, momentum offset, energies and every time step.'], 'nbody-toit-8.toit')


app('knucleotide', 'packed', r'''
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
''', ['Preserve all seven growing hash maps and every overlapping window; exact 36-bit ACGT keys use native signed 64-bit integers without aliasing long windows. Buffered general FASTA input and deterministic frequency ties.'], 'knucleotide-toit-8.toit')


ALU = r'''"""
    GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG\
    GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA\
    CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT\
    ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA\
    GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG\
    AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC\
    AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA\
    """'''
app('fasta', 'streaming', r'''
import io.stdio
import bytes show Buffer

ALU ::= ALU_LITERAL
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
'''.replace('ALU_LITERAL', ALU), ['Retain every sequential LCG advance, direct binary64 division and unscaled cumulative probabilities; general binary-search selection and reusable row/block buffers reduce allocation and native writes. Complete three-record output.'], 'fasta-toit-1.toit')


REVCOMP = r'''
import io.stdio
import bytes show Buffer

complements -> ByteArray:
  lookup := ByteArray 256: it
  symbols := "ABCDGHKMNRSTUVWY"
  values := "TVGHCDMKNYSAABWR"
  symbols.size.repeat:
    lookup[symbols[it]] = values[it]
    lookup[symbols[it] + 32] = values[it]
  return lookup

emit header/string? sequence/Buffer lookup/ByteArray:
  if not header: return
  print header
  data := sequence.bytes
  front := 0
  back := data.size - 1
  while front <= back:
    saved := lookup[data[front]]
    data[front] = lookup[data[back]]
    data[back] = saved
    front++
    back--
  EMIT_ROWS

main args:
  lookup := complements
  header := null
  sequence := Buffer
  while line := stdio.stdin.read-line:
    if line.starts-with ">":
      emit header sequence lookup
      header = line
      sequence.clear
    else if not (line.starts-with ";"):
      sequence.write line
  emit header sequence lookup
'''
app('revcomp', 'lines', REVCOMP.replace('  EMIT_ROWS', '''  for offset := 0; offset < data.size; offset += 60:
    stdio.stdout.write data offset (min data.size (offset + 60))
    print'''), ['Replace the published scanner that misses multiple headers in a read with general FASTA record parsing, arbitrary headers/line lengths/comments/CRLF, in-place reverse/complement and canonical 60-column complete output. All parsing and transformation remain counted Toit source.'], 'revcomp-toit-1.toit')
app('revcomp', 'buffered', REVCOMP.replace('  EMIT_ROWS', '''  output := Buffer
  for offset := 0; offset < data.size; offset += 60:
    output.write data offset (min data.size (offset + 60))
    output.write-byte '\\n'
    if output.size >= 65536:
      stdio.stdout.write output.bytes
      output.clear
  stdio.stdout.write output.bytes'''), ['Same general FASTA parser and full in-place byte reversal/complement as the line-output form; standard block Buffer batches canonical output.'], 'revcomp-toit-1.toit')


PI = r'''
import bignum show Bignum

SMALL_CONVERSION

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
    if IS_NEGATIVE: continue
    digit := (q * 3 + r) / t
    if digit != (q * 4 + r) / t: continue
    value := SMALL_VALUE
    line += "$value"
    emitted++
    if emitted % 10 == 0 or emitted == n:
      print "$line$(" " * (10 - line.size))\t:$emitted"
      line = ""
    r = (r - t * digit) * 10
    q = q * 10
'''
app('pidigits', 'bignum', PI.replace('SMALL_CONVERSION', '').replace('IS_NEGATIVE', '((r - q).stringify.starts-with "-")').replace('SMALL_VALUE', '(digit == 0 ? 0 : (int.parse digit.stringify))'), ['Author the required unbounded-integer streaming spigot using SDK Bignum. Both extract(3) and extract(4) use exact big-integer division at every eligible iteration; no precomputation, factor cancellation or bounded integer substitution.'])
app('pidigits', 'bignum-fields', PI.replace('SMALL_CONVERSION', '''small-integer value/Bignum -> int:
  result := 0
  value.limbs_.do: result = (result << 8) | it
  return value.negative_ ? -result : result''').replace('IS_NEGATIVE', '(r - q).negative_').replace('SMALL_VALUE', '(small-integer digit)'), ['Same strict unbounded-integer spigot and both exact extract divisions; use the pinned standard Bignum sign and byte representation for general sign/small-integer conversion rather than allocating hexadecimal strings. All local helpers are counted.'])


app('regexredux', 'regexp', r'''
import io.stdio
import io show Buffer
import regexp show RegExp

replace-all pattern/string replacement/string subject/string -> string:
  expression := RegExp pattern
  result := Buffer
  offset := 0
  expression.all-matches subject 0: | match |
    result.write subject offset match.index
    result.write replacement
    offset = match.end-index
  result.write subject offset subject.size
  return result.bytes.to-string

main args:
  raw := stdio.stdin.read-all
  subject := raw ? raw.to-string : ""
  input-size := subject.size
  subject = replace-all ">[^\n]*\n|\n" "" subject
  cleaned-size := subject.size
  ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"].do: | pattern |
    count := 0
    (RegExp pattern).all-matches subject 0: count++
    print "$pattern $count"
  [["tHa[Nt]", "<4>"], ["aND|caN|Ha[DS]|WaS", "<3>"], ["a[NSt]|BY", "<2>"], ["<[^>]*>", "|"], ["\\|[^|][^|]*\\|", "-"]].do:
    subject = replace-all it[0] it[1] subject
  print "\n$input-size\n$cleaned-size\n$(subject.size)"
''', ['Author regex-redux using the pinned general dartino_regexp 0.4.1 JavaScript-compatible regex package, with native buffered stdin, all nine independent nonoverlapping counts and five sequential whole-sequence substitutions. Complete lengths and output; no specialized motif recognizer.'])

(D/'support/toit/custom-candidates.json').write_text(json.dumps(rows, indent=2)+'\n')
print('WROTE', len(rows), 'custom Toit candidates')
