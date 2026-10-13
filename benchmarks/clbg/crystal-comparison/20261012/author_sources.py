"""Write editable pure-Crystal drafts; no compiler, runtime, hash or scoring."""
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
PROFILES=[]
WORK={
    'nbody':dict(bodies=5,pairs_per_step=10,position_updates_per_step=5,before_after_energy=True,exact_sqrt_division=True),
    'mandelbrot':dict(maximum_actual_recurrences=50,original_dimensions=True,full_binary_pbm=True,zero_tail_bits=True),
    'spectralnorm':dict(power_iterations=10,fresh_implicit_products=40,matrix_cache=False),
    'binarytrees':dict(distinct_internal_and_leaf_nodes=True,long_lived_root_retained=True,scoped_stretch=True,checksum_formula=False),
    'fannkuchredux':dict(all_factorial_ranks=True,global_rank_parity=True,actual_prefix_flips=True,permutation_cache=False),
    'fasta':dict(record_lengths=['2*n','3*n','5*n'],binary64_division_per_draw=True,ordered_unscaled_cdfs=True,actual_carried_lcg_state=True,line_width=60,generated_letter_cache=False),
    'revcomp':dict(all_records=True,complete_iupac=True,comments_cr_lf_eof=True,line_width=60),
    'knucleotide':dict(growable_maps=7,widths=[1,2,3,4,6,12,18],all_overlapping_windows=True,first_two_are_maps=True,three_record_only=True,comments_cr_lf_eof=True),
    'regexredux':dict(nonoverlapping_patterns=9,sequential_ordered_replacement_passes=5,complete_stdin=True),
    'pidigits':dict(two_actual_arbitrary_precision_divisions_per_candidate=True,distinct_endpoint_numerators=True,complete_digit_rows_tail_count=True,zero_digit_empty=True),
}

def source(benchmark,variant,text,ancestry,technique,workers=1):
    path=HERE/'sources'/benchmark/(variant+'.cr');path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(text.strip()+'\n')
    PROFILES.append(dict(id='crystal-'+benchmark+'-'+variant,benchmark=benchmark,variant=variant,
        source=path.relative_to(HERE).as_posix(),status='authored-unbuilt-unqualified',
        source_sha256=None,loc=None,canonical_loc_status='pending common normalization audit',
        upstream_provenance=ancestry,algorithm=dict(strict=True,native_benchmark_kernel=False,
        required_work_review='pending bounded native qualification and independent traces',required_work=WORK[benchmark]),
        required_work=WORK[benchmark],
        maximum_workers=workers,technique=technique,compiler_flags=['--release','--no-debug','--mcpu','native'],
        timing_samples=[],chart_eligible=False))

def upstream(family,path):
    return dict(repository='https://github.com/hanabi1224/Programming-Language-Benchmarks' if family=='hanabi' else 'https://github.com/kostya/crystal-benchmarks-game',
        commit='ad18b203dd1769724f4eea94fc3ac1e99f6593e0' if family=='hanabi' else 'aaf7bd44118889605943df3200b7c9b0c6af6510',
        path=path,kind='locally-adapted-published-algorithm; original remains immutable')

BODY_DATA=r'''
SOLAR_MASS = 4.0 * Math::PI * Math::PI
DAYS_PER_YEAR = 365.24
def initial_bodies
  [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS],
    [4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01, 1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR, 9.54791938424326609e-04 * SOLAR_MASS],
    [8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01, -2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR, 2.85885980666130812e-04 * SOLAR_MASS],
    [1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01, 2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR, 4.36624404335156298e-05 * SOLAR_MASS],
    [1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01, 2.68067772490389322e-03 * DAYS_PER_YEAR, 1.62824170038242295e-03 * DAYS_PER_YEAR, -9.51592254519715870e-05 * DAYS_PER_YEAR, 5.15138902046611451e-05 * SOLAR_MASS],
  ]
end
'''
source('nbody','compact',BODY_DATA+r'''
def energy(b)
  total = 0.0
  b.size.times do |i|
    total += 0.5 * b[i][6] * (b[i][3] ** 2 + b[i][4] ** 2 + b[i][5] ** 2)
    ((i + 1)...b.size).each do |j|
      dx = b[i][0] - b[j][0]
      dy = b[i][1] - b[j][1]
      dz = b[i][2] - b[j][2]
      total -= b[i][6] * b[j][6] / Math.sqrt(dx * dx + dy * dy + dz * dz)
    end
  end
  total
end
n = (ARGV[0]? || "1000").to_i
b = initial_bodies
(3...6).each do |coordinate|
  momentum = 0.0
  b.each { |body| momentum += body[coordinate] * body[6] }
  b[0][coordinate] = -momentum / SOLAR_MASS
end
puts "%.9f" % energy(b)
n.times do
  b.size.times do |i|
    ((i + 1)...b.size).each do |j|
      dx = b[i][0] - b[j][0]
      dy = b[i][1] - b[j][1]
      dz = b[i][2] - b[j][2]
      squared = dx * dx + dy * dy + dz * dz
      magnitude = 0.01 / (squared * Math.sqrt(squared))
      displacement = StaticArray[dx, dy, dz]
      3.times do |coordinate|
        b[i][coordinate + 3] -= displacement[coordinate] * b[j][6] * magnitude
        b[j][coordinate + 3] += displacement[coordinate] * b[i][6] * magnitude
      end
    end
  end
  b.each do |body|
    3.times { |coordinate| body[coordinate] += 0.01 * body[coordinate + 3] }
  end
end
puts "%.9f" % energy(b)
''',upstream('hanabi','bench/algorithm/nbody/1.cr'),'Ordinary body coordinate arrays, stack displacement, exact sqrt/division and complete pair/position loops.')
body_fast=(HERE/'research/upstream/hanabi/bench/algorithm/nbody/1.cr').read_text()
body_fast=body_fast.replace('@x, @y, @z = x, y, z','@x = x\n    @y = y\n    @z = z')
body_fast=body_fast.replace('@vx, @vy, @vz = Float64.new(vx * DAYS_PER_YEAR), Float64.new(vy * DAYS_PER_YEAR), Float64.new(vz * DAYS_PER_YEAR)','@vx = Float64.new(vx * DAYS_PER_YEAR)\n    @vy = Float64.new(vy * DAYS_PER_YEAR)\n    @vz = Float64.new(vz * DAYS_PER_YEAR)')
body_fast=body_fast.replace('b_mass_mag, b2_mass_mag = mass * mag, b2.mass * mag','b_mass_mag = mass * mag\n      b2_mass_mag = b2.mass * mag')
body_fast=body_fast.replace('px, py, pz = 0.0, 0.0, 0.0','px = 0.0\n  py = 0.0\n  pz = 0.0')
source('nbody','optimized',body_fast,
    upstream('hanabi','bench/algorithm/nbody/1.cr'),'Published scalar cached body fields and triangular pair loops, exact sqrt/division.')

source('mandelbrot','compact',r'''
n = (ARGV[0]? || "200").to_i
raise ArgumentError.new("Negative size") if n < 0
STDOUT.print("P4\n#{n} #{n}\n")
scale = 2.0 / n
n.times do |y|
  byte = 0_u8
  bits = 0
  n.times do |x|
    cr = x * scale - 1.5
    ci = y * scale - 1.0
    zr = 0.0
    zi = 0.0
    rr = 0.0
    ii = 0.0
    iterations = 0
    while iterations < 50 && rr + ii <= 4.0
      zi = 2.0 * zr * zi + ci
      zr = rr - ii + cr
      rr = zr * zr
      ii = zi * zi
      iterations += 1
    end
    byte = (byte << 1) | (rr + ii <= 4.0 ? 1_u8 : 0_u8)
    bits += 1
    if bits == 8 || x == n - 1
      STDOUT.write_byte(byte << (8 - bits))
      byte = 0_u8
      bits = 0
    end
  end
end
''',upstream('kostya','mandelbrot/mandelbrot.cr'),'Scalar escape recurrence and exact-width complete PBM with zero tail bits.')
source('mandelbrot','optimized',r'''
alias Lane = StaticArray(Float64, 8)
def pixel_byte(cr : Lane, ci : Float64, valid : Int32) : UInt8
  zr = Lane.new(0.0)
  zi = Lane.new(0.0)
  rr = Lane.new(0.0)
  ii = Lane.new(0.0)
  10.times do
    5.times do
      8.times do |lane|
        zi[lane] = 2.0 * zr[lane] * zi[lane] + ci
        zr[lane] = rr[lane] - ii[lane] + cr[lane]
        rr[lane] = zr[lane] * zr[lane]
        ii[lane] = zi[lane] * zi[lane]
      end
    end
    escaped = true
    valid.times { |lane| escaped = false if rr[lane] + ii[lane] <= 4.0 }
    return 0_u8 if escaped
  end
  byte = 0_u8
  valid.times { |lane| byte |= (0x80_u8 >> lane) if rr[lane] + ii[lane] <= 4.0 }
  byte
end
def rows(n : Int32, first : Int32, last : Int32, done : Channel(Tuple(Int32, Bytes)))
  stride = (n + 7) // 8
  output = Bytes.new((last - first) * stride, 0_u8)
  scale = 2.0 / n
  (first...last).each do |y|
    stride.times do |column|
      valid = Math.min(8, n - column * 8)
      cr = Lane.new { |lane| (column * 8 + lane) * scale - 1.5 }
      output[(y - first) * stride + column] = pixel_byte(cr, y * scale - 1.0, valid)
    end
  end
  done.send({first, output})
end
n = (ARGV[0]? || "200").to_i
raise ArgumentError.new("Negative size") if n < 0
STDOUT.print("P4\n#{n} #{n}\n")
if n > 0
  Fiber::ExecutionContext.default.resize(8)
  workers = Math.min(8, n)
  done = Channel(Tuple(Int32, Bytes)).new(workers)
  workers.times do |worker|
    spawn rows(n, n * worker // workers, n * (worker + 1) // workers, done)
  end
  blocks = Array(Tuple(Int32, Bytes)).new(workers) { done.receive }
  blocks.sort_by! { |block| block[0] }
  blocks.each { |block| STDOUT.write(block[1]) }
end
''',upstream('hanabi','bench/algorithm/mandelbrot/2.cr'),'Eight standard row fibers, fixed eight-lane arrays/five-step escape blocks, exact dimensions and full PBM.',8)

source('spectralnorm','compact',r'''
def apply(v, transpose = false)
  Array.new(v.size) do |i|
    sum = 0.0
    v.each_with_index do |value, j|
      row = transpose ? j : i
      column = transpose ? i : j
      sum += value / ((row + column) * (row + column + 1) // 2 + row + 1)
    end
    sum
  end
end
n = (ARGV[0]? || "100").to_i
raise ArgumentError.new("Positive size required") if n <= 0
u = Array.new(n, 1.0)
v = Array.new(n, 0.0)
10.times do
  v = apply(apply(u), true)
  u = apply(apply(v), true)
end
uv = 0.0
vv = 0.0
n.times do |i|
  uv += u[i] * v[i]
  vv += v[i] * v[i]
end
puts "%.9f" % Math.sqrt(uv / vv)
''',upstream('kostya','spectralnorm/spectralnorm.cr'),'Forty fresh implicit coefficient products and nine-decimal output.')
source('spectralnorm','optimized',r'''
def row_block(v : Array(Float64), result : Array(Float64), first : Int32, last : Int32, transpose : Bool, done : Channel(Nil))
  n = v.size
  (first...last).each do |i|
    denominator = transpose ? (i * (i + 1) // 2 + 1) : (i * (i + 1) // 2 + i + 1)
    a = 0.0
    b = 0.0
    c = 0.0
    d = 0.0
    j = 0
    while j + 3 < n
      a += v[j] / denominator
      denominator += i + j + (transpose ? 2 : 1)
      b += v[j + 1] / denominator
      denominator += i + j + (transpose ? 3 : 2)
      c += v[j + 2] / denominator
      denominator += i + j + (transpose ? 4 : 3)
      d += v[j + 3] / denominator
      denominator += i + j + (transpose ? 5 : 4)
      j += 4
    end
    sum = (a + b) + (c + d)
    while j < n
      sum += v[j] / denominator
      denominator += i + j + (transpose ? 2 : 1)
      j += 1
    end
    result[i] = sum
  end
  done.send(nil)
end
def apply(v : Array(Float64), transpose = false)
  n = v.size
  workers = Math.min(8, n)
  result = Array.new(n, 0.0)
  done = Channel(Nil).new(workers)
  workers.times { |worker| spawn row_block(v, result, n * worker // workers, n * (worker + 1) // workers, transpose, done) }
  workers.times { done.receive }
  result
end
n = (ARGV[0]? || "100").to_i
raise ArgumentError.new("Positive size required") if n <= 0
Fiber::ExecutionContext.default.resize(8)
u = Array.new(n, 1.0)
v = Array.new(n, 0.0)
10.times do
  v = apply(apply(u), true)
  u = apply(apply(v), true)
end
uv = 0.0
vv = 0.0
n.times do |i|
  uv += u[i] * v[i]
  vv += v[i] * v[i]
end
puts "%.9f" % Math.sqrt(uv / vv)
''',upstream('hanabi','bench/algorithm/spectral-norm/1.cr'),'Eight disjoint row fibers, exact denominator recurrence, four accumulation chains and scalar tails; no coefficient cache.',8)

TREE=r'''
class Node
  property left : Node?
  property right : Node?
  def initialize(@left : Node? = nil, @right : Node? = nil)
  end
  def check : Int64
    left = @left
    right = @right
    left && right ? 1_i64 + left.check + right.check : 1_i64
  end
end
def tree(depth : Int32) : Node
  depth == 0 ? Node.new : Node.new(tree(depth - 1), tree(depth - 1))
end
def stretch_check(depth)
  tree(depth).check
end
def check_many(depth, iterations)
  total = 0_i64
  iterations.times { total += tree(depth).check }
  total
end
'''
source('binarytrees','compact',TREE+r'''
maximum = Math.max(6, (ARGV[0]? || "10").to_i)
puts "stretch tree of depth #{maximum + 1}\t check: #{stretch_check(maximum + 1)}"
long_lived = tree(maximum)
4.step(to: maximum, by: 2) do |depth|
  iterations = 1_i64 << (maximum - depth + 4)
  puts "#{iterations}\t trees of depth #{depth}\t check: #{check_many(depth, iterations)}"
end
puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"
''',upstream('hanabi','bench/algorithm/binarytrees/1.cr'),'Separate class Node allocation including every leaf; scoped stretch root and retained long-lived root.')
source('binarytrees','optimized',TREE+r'''
def tree_job(depth : Int32, iterations : Int64, done : Channel(Tuple(Int32, Int64)))
  done.send({depth, check_many(depth, iterations)})
end
maximum = Math.max(6, (ARGV[0]? || "10").to_i)
puts "stretch tree of depth #{maximum + 1}\t check: #{stretch_check(maximum + 1)}"
long_lived = tree(maximum)
Fiber::ExecutionContext.default.resize(8)
depths = [] of Int32
4.step(to: maximum, by: 2) { |depth| depths << depth }
depths.each_slice(8) do |batch|
  done = Channel(Tuple(Int32, Int64)).new(batch.size)
  batch.each do |depth|
    spawn tree_job(depth, 1_i64 << (maximum - depth + 4), done)
  end
  results = Array(Tuple(Int32, Int64)).new(batch.size) { done.receive }
  results.sort_by! { |record| record[0] }
  results.each do |record|
    depth = record[0]
    iterations = 1_i64 << (maximum - depth + 4)
    puts "#{iterations}\t trees of depth #{depth}\t check: #{record[1]}"
  end
end
puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"
''',upstream('hanabi','bench/algorithm/binarytrees/1.cr'),'Same distinct nodes/traversals in bounded batches of at most eight depth fibers; retained real long-lived root.',8)

fann_compact=(HERE/'research/upstream/hanabi/bench/algorithm/fannkuch-redux/1.cr').read_text()
fann_compact=fann_compact.replace('maxFlipsCount = permCount = checksum = 0','maxFlipsCount = 0\n  permCount = 0_i64\n  checksum = 0_i64')
fann_compact=fann_compact.replace('sum, flips = fannkuchredux(n)','raise ArgumentError.new("Size must be 1..20") unless n >= 1 && n <= 20\nsum, flips = fannkuchredux(n)')
source('fannkuchredux','compact',fann_compact,
    upstream('hanabi','bench/algorithm/fannkuch-redux/1.cr'),'Published fixed-array sequential permutation rotations and actual prefix flips/global parity.')
source('fannkuchredux','optimized',r'''
def rank_range(n : Int32, first : Int64, last : Int64) : Tuple(Int64, Int32)
  factorial = StaticArray(Int64, 21).new(1_i64)
  (1..n).each { |width| factorial[width] = factorial[width - 1] * width }
  permutation = StaticArray(Int32, 32).new { |i| i }
  rotations = StaticArray(Int32, 32).new(0)
  n.downto(2) do |width|
    shift = ((first // factorial[width - 1]) % width).to_i
    rotations[width - 1] = shift
    shift.times do
      initial = permutation[0]
      (width - 1).times { |j| permutation[j] = permutation[j + 1] }
      permutation[width - 1] = initial
    end
  end
  checksum = 0_i64
  maximum = 0
  (first...last).each do |rank|
    work = permutation
    flips = 0
    while work[0] != 0
      left = 0
      right = work[0]
      while left < right
        temporary = work[left]
        work[left] = work[right]
        work[right] = temporary
        left += 1
        right -= 1
      end
      flips += 1
    end
    checksum += rank.even? ? flips : -flips
    maximum = Math.max(maximum, flips)
    (2..n).each do |width|
      initial = permutation[0]
      (width - 1).times { |j| permutation[j] = permutation[j + 1] }
      permutation[width - 1] = initial
      rotations[width - 1] += 1
      break if rotations[width - 1] < width
      rotations[width - 1] = 0
    end
  end
  {checksum, maximum}
end
def range_job(n : Int32, first : Int64, last : Int64, done : Channel(Tuple(Int64, Int32)))
  done.send(rank_range(n, first, last))
end
n = (ARGV[0]? || "10").to_i
raise ArgumentError.new("Size must be 1..20") unless n >= 1 && n <= 20
total = 1_i64
(1..n).each { |width| total *= width }
workers = Math.min(8_i64, total).to_i
done = Channel(Tuple(Int64, Int32)).new(workers)
Fiber::ExecutionContext.default.resize(8)
workers.times do |worker|
  first = (total // workers) * worker + Math.min(worker.to_i64, total % workers)
  last = (total // workers) * (worker + 1) + Math.min((worker + 1).to_i64, total % workers)
  spawn range_job(n, first, last, done)
end
checksum = 0_i64
maximum = 0
workers.times do
  result = done.receive
  checksum += result[0]
  maximum = Math.max(maximum, result[1])
end
puts checksum
puts "Pfannkuchen(#{n}) = #{maximum}"
''',upstream('hanabi','bench/algorithm/fannkuch-redux/1.cr'),'Eight standard fibers over complete factoradic rank intervals; real flips/global parity and overflow-safe boundaries.',8)

source('fasta','compact',(HERE/'research/upstream/hanabi/bench/algorithm/fasta/1.cr').read_text(),
    upstream('hanabi','bench/algorithm/fasta/1.cr'),'Published ordered floating CDF scan and carried actual sequential binary64 RNG draws.')
fasta_fast=(HERE/'research/upstream/hanabi/bench/algorithm/fasta/2.cr').read_text()
fasta_fast=fasta_fast.replace('puts ">#{id} #{desc}"','emit_line(">#{id} #{desc}")').replace('puts(pick)','emit_line(pick)').replace('print(s[k..-1])','emit(s[k..-1])').replace('puts(s[k...k + m])','emit_line(s[k...k + m])')
source('fasta','optimized',r'''
OUTPUT = IO::Memory.new(64 * 1024)
def emit(text : String)
  OUTPUT << text
  if OUTPUT.bytesize >= 64 * 1024
    STDOUT.write(OUTPUT.to_slice)
    OUTPUT.clear
  end
end
def emit_line(text : String)
  emit(text)
  emit("\n")
end
'''+fasta_fast+r'''
STDOUT.write(OUTPUT.to_slice)
''',upstream('hanabi','bench/algorithm/fasta/2.cr'),'Published array CDF binary search with bounded buffered writes; direct binary64 division for every actual draw and carried seed.')

source('revcomp','compact',r'''
def reverse_complement(sequence : String)
  output = sequence.reverse.tr("ACGTUMRWSYKVHDBNacgtumrwsykvhdbn", "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN")
  0.step(to: output.bytesize - 1, by: 60) { |start| puts output.byte_slice(start, Math.min(60, output.bytesize - start)) }
end
header = false
sequence = String::Builder.new
STDIN.gets_to_end.split(/\r\n|\r|\n/).each do |line|
  next if line.starts_with?(";")
  if line.starts_with?(">")
    reverse_complement(sequence.to_s) if header
    sequence = String::Builder.new
    puts line
    header = true
  elsif header
    sequence << line
  end
end
reverse_complement(sequence.to_s) if header
''',upstream('kostya','revcomp/revcomp.cr'),'General String reversal/IUPAC translation, all records including empty headers, comment/CRLF/EOF repair and 60-column output.')
source('revcomp','optimized',r'''
COMPLEMENT = begin
  table = StaticArray(UInt8, 256).new { |index| index.to_u8 }
  letters = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
  values = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"
  letters.bytes.each_with_index { |value, index| table[value] = values.byte_at(index) }
  table
end
def write_complement(sequence : Array(UInt8))
  left = 0
  right = sequence.size - 1
  while left <= right
    a = sequence[left]
    b = sequence[right]
    sequence[left] = COMPLEMENT[b]
    sequence[right] = COMPLEMENT[a]
    left += 1
    right -= 1
  end
  output = IO::Memory.new(64 * 1024)
  offset = 0
  while offset < sequence.size
    count = Math.min(60, sequence.size - offset)
    output.write(Slice.new(sequence.to_unsafe, sequence.size)[offset, count])
    output.write_byte(10_u8)
    offset += count
    if output.bytesize >= 64 * 1024
      STDOUT.write(output.to_slice)
      output.clear
    end
  end
  STDOUT.write(output.to_slice)
end
input = STDIN.gets_to_end
bytes = input.to_slice
position = 0
header = false
sequence = [] of UInt8
while position < bytes.size
  first = position
  while position < bytes.size && bytes[position] != 10_u8 && bytes[position] != 13_u8
    position += 1
  end
  last = position
  if position < bytes.size
    separator = bytes[position]
    position += 1
    position += 1 if separator == 13_u8 && position < bytes.size && bytes[position] == 10_u8
  end
  next if first == last || bytes[first] == 59_u8
  if bytes[first] == 62_u8
    write_complement(sequence) if header
    sequence.clear
    STDOUT.write(bytes[first, last - first])
    STDOUT.write_byte(10_u8)
    header = true
  elsif header
    (first...last).each { |index| sequence << bytes[index] }
  end
end
write_complement(sequence) if header
''',upstream('kostya','revcomp/revcomp.cr'),'Byte parser, standard complement table, in-place reversal/translation and bounded writes; all record/comment/EOF semantics.')

DNA=r'''
LENGTHS = [1, 2, 3, 4, 6, 12, 18]
QUERIES = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
def third_record
  selected = false
  sequence = String::Builder.new
  STDIN.gets_to_end.split(/\r\n|\r|\n/).each do |line|
    next if line.starts_with?(";")
    if line.starts_with?(">")
      break if selected
      selected = line == ">THREE" || line.starts_with?(">THREE ") || line.starts_with?(">THREE\t")
    elsif selected
      sequence << line.upcase
    end
  end
  sequence.to_s
end
def frequencies(table : Hash(String, Int64))
  total = table.values.sum(0_i64)
  rows = table.to_a.sort do |left, right|
    left[1] == right[1] ? left[0] <=> right[0] : right[1] <=> left[1]
  end
  rows.each { |row| puts "#{row[0]} #{"%.3f" % (100.0 * row[1] / total)}" }
  puts ""
end
'''
source('knucleotide','compact',DNA+r'''
sequence = third_record
tables = LENGTHS.map do |width|
  counts = Hash(String, Int64).new(0_i64)
  Math.max(0, sequence.bytesize - width + 1).times do |start|
    counts[sequence.byte_slice(start, width)] += 1
  end
  counts
end
2.times { |index| frequencies(tables[index]) }
QUERIES.each_with_index { |query, index| puts "#{tables[index + 2][query]}\t#{query}" }
''',upstream('kostya','knucleotide/knucleotide.cr'),'Seven full growable string-key maps, all windows including 1/2; exact target parser and deterministic frequencies.')
source('knucleotide','optimized',DNA+r'''
def code(value : UInt8) : UInt64
  case value
  when 65_u8 then 0_u64
  when 67_u8 then 1_u64
  when 71_u8 then 2_u64
  when 84_u8 then 3_u64
  else raise ArgumentError.new("Invalid nucleotide")
  end
end
def decode(key : UInt64, width : Int32) : String
  output = Bytes.new(width)
  remaining = key
  (width - 1).downto(0) do |index|
    output[index] = "ACGT".byte_at((remaining & 3_u64).to_i)
    remaining >>= 2
  end
  String.new(output)
end
def histogram(sequence : String, width : Int32, index : Int32, done : Channel(Tuple(Int32, Hash(UInt64, Int64))))
  counts = Hash(UInt64, Int64).new(0_i64)
  mask = (1_u64 << (2 * width)) - 1_u64
  key = 0_u64
  sequence.to_slice.each_with_index do |value, position|
    key = ((key << 2) | code(value)) & mask
    counts[key] += 1 if position + 1 >= width
  end
  done.send({index, counts})
end
sequence = third_record
Fiber::ExecutionContext.default.resize(8)
done = Channel(Tuple(Int32, Hash(UInt64, Int64))).new(7)
LENGTHS.each_with_index { |width, index| spawn histogram(sequence, width, index, done) }
tables = Array.new(7) { Hash(UInt64, Int64).new(0_i64) }
7.times do
  record = done.receive
  tables[record[0]] = record[1]
end
2.times do |index|
  rows = Hash(String, Int64).new(0_i64)
  tables[index].each { |key, value| rows[decode(key, LENGTHS[index])] = value }
  frequencies(rows)
end
QUERIES.each_with_index do |query, index|
  key = 0_u64
  query.to_slice.each { |value| key = (key << 2) | code(value) }
  puts "#{tables[index + 2][key]}\t#{query}"
end
''',upstream('kostya','knucleotide/knucleotide.cr'),'Seven ordinary growable rolling packed-key maps in capped standard fibers, all retained through parent output.',7)

REGEX=r'''
PATTERNS = [
  /agggtaaa|tttaccct/,
  /[cgt]gggtaaa|tttaccc[acg]/,
  /a[act]ggtaaa|tttacc[agt]t/,
  /ag[act]gtaaa|tttac[agt]ct/,
  /agg[act]taaa|ttta[agt]cct/,
  /aggg[acg]aaa|ttt[cgt]ccct/,
  /agggt[cgt]aa|tt[acg]accct/,
  /agggta[cgt]a|t[acg]taccct/,
  /agggtaa[cgt]|[acg]ttaccct/,
]
SUBSTITUTIONS = [
  {/tHa[Nt]/, "<4>"},
  {/aND|caN|Ha[DS]|WaS/, "<3>"},
  {/a[NSt]|BY/, "<2>"},
  {/<[^>]*>/, "|"},
  {/\|[^|][^|]*\|/, "-"},
]
def count_matches(sequence : String, pattern : Regex)
  count = 0
  position = 0
  while match = pattern.match(sequence, position)
    count += 1
    position = match.end
  end
  count
end
input = STDIN.gets_to_end
original_length = input.bytesize
sequence = input.gsub(/>[^\n]*\n|\n/, "")
cleaned_length = sequence.bytesize
'''
REGEX_END=r'''
SUBSTITUTIONS.each { |pair| sequence = sequence.gsub(pair[0], pair[1]) }
puts ""
puts original_length
puts cleaned_length
puts sequence.bytesize
'''
source('regexredux','compact',REGEX+r'''
PATTERNS.each { |pattern| puts "#{pattern.source} #{count_matches(sequence, pattern)}" }
'''+REGEX_END,upstream('hanabi','bench/algorithm/regex-redux/1.cr'),'Standard PCRE2 nonoverlapping matching and five ordered replacements, complete stdin/output.')
source('regexredux','optimized',REGEX+r'''
def count_job(sequence : String, source : String, index : Int32, done : Channel(Tuple(Int32, Int32)))
  pattern = Regex.new(source)
  done.send({index, count_matches(sequence, pattern)})
end
Fiber::ExecutionContext.default.resize(8)
done = Channel(Tuple(Int32, Int32)).new(8)
8.times { |index| spawn count_job(sequence, PATTERNS[index].source, index, done) }
counts = Array.new(9, 0)
8.times do
  record = done.receive
  counts[record[0]] = record[1]
end
counts[8] = count_matches(sequence, PATTERNS[8])
PATTERNS.each_with_index { |pattern, index| puts "#{pattern.source} #{counts[index]}" }
'''+REGEX_END,upstream('hanabi','bench/algorithm/regex-redux/1.cr'),'Eight independent standard PCRE2 count fibers/private regex state, ninth after completion; five replacements remain sequential.',8)

source('pidigits','compact',r'''
require "big"
n = (ARGV[0]? || "27").to_i
q = BigInt.new(1)
r = BigInt.new(0)
t = BigInt.new(1)
k = 1
printed = 0
line = String::Builder.new
while printed < n
  numerator3 = q * 3 + r
  numerator4 = q * 4 + r
  digit3 = numerator3 // t
  digit4 = numerator4 // t
  if digit3 == digit4
    line << digit3.to_s
    printed += 1
    if printed % 10 == 0 || printed == n
      line << " " * ((10 - printed % 10) % 10)
      puts "#{line.to_s}\t:#{printed}"
      line = String::Builder.new
    end
    r = (r - digit3 * t) * 10
    q *= 10
  else
    factor = 2 * k + 1
    r = (q * 2 + r) * factor
    q *= k
    t *= factor
    k += 1
  end
end
''',upstream('hanabi','bench/algorithm/pidigits/1.cr'),'Standard GMP BigInt q/r/t LFT, two real endpoint divisions every tested candidate, complete row/tail/count output.')
source('pidigits','optimized',r'''
require "big"
n = (ARGV[0]? || "27").to_i
upper = BigInt.new(4)
lower = BigInt.new(3)
denominator = BigInt.new(1)
k = 1
printed = 0
line = String::Builder.new
while printed < n
  digit4 = upper // denominator
  digit3 = lower // denominator
  if digit3 == digit4
    line << digit3.to_s
    printed += 1
    if printed % 10 == 0 || printed == n
      line << " " * ((10 - printed % 10) % 10)
      puts "#{line.to_s}\t:#{printed}"
      line = String::Builder.new
    end
    subtract = digit3 * denominator * 10
    upper = upper * 10 - subtract
    lower = lower * 10 - subtract
  else
    next_upper = upper * (2 * k - 1) + lower * 2
    next_lower = upper * (k - 1) + lower * (k + 2)
    upper = next_upper
    lower = next_lower
    denominator *= 2 * k + 1
    k += 1
  end
end
''',upstream('hanabi','bench/algorithm/pidigits/1.cr'),'Pure Crystal cached endpoint-numerator basis with standard scalar GMP multipliers; both actual BigInt divisions and full output.')

if __name__=='__main__':
    (HERE/'profiles.json').write_text(json.dumps(dict(status='source-drafts-unbuilt-unqualified',version='1.21.1',
        compiler_source_commit='73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef',profiles=PROFILES,
        complete_workloads=sorted({p['benchmark'] for p in PROFILES}),builds=0,probes=0),indent=2)+'\n')
