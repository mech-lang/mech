"""Write reviewable V application drafts only; never build or run workloads."""
import json
from pathlib import Path
HERE=Path(__file__).resolve().parent
UP=HERE/'upstream/hanabi/bench/algorithm'
profiles=[]
def write(benchmark,role,text,*,upstream=None,technique='',work=None,library='V standard library'):
    path=HERE/'sources'/benchmark/(role+'.v');path.parent.mkdir(parents=True,exist_ok=True)
    # Template escapes are doubled for readability; emit ordinary V escapes.
    path.write_text(text.strip().replace(chr(92) * 2,chr(92))+'\n')
    workers=7 if role=='optimized' and benchmark=='knucleotide' else (8 if role=='optimized' and benchmark in ('mandelbrot','spectralnorm','binarytrees','fannkuchredux','regexredux') else 1)
    profiles.append(dict(id='v-'+benchmark+'-'+role,benchmark=benchmark,variant=role,role=role,source=str(path.relative_to(HERE)),source_sha256=None,loc=None,loc_status='pending canonical normalization under common mutex',status='authored-unbuilt-unqualified',provenance=dict(kind='adapted-published-source' if upstream else 'locally-authored-V',upstream=str(upstream.relative_to(HERE)) if upstream else None,upstream_repository='https://github.com/hanabi1224/Programming-Language-Benchmarks',upstream_commit='ad18b203dd1769724f4eea94fc3ac1e99f6593e0'),technique=technique,library=library,required_work=work or {},algorithm=dict(status='source-draft-awaiting-independent-review',strict=True,native_benchmark_kernel=False,reason=technique,required_work=work or {}),native_benchmark_kernel=False,maximum_workers=workers,maximum_compute_workers=workers,compiler_flags=['-prod','-cc','/opt/homebrew/opt/llvm@22/bin/clang-22','-cflags','-O3 -march=native -ffp-contract=off'],chart_eligible=False,report_only=True))
    if role=='published-package':
        profiles[-1]['library_provenance']=dict(repository='https://github.com/hanabi1224/vlang-big-integer',commit='dae1213e3b7576b40bf84a5c45a25748a74ef3c0',original_sources='upstream/vlang-big-integer',compatibility_status='unbuilt; old option/error syntax needs release compatibility review')
HEADER='module main\n\nimport os\nimport strconv\n'
ARG="""fn size(default_value int) int {
    if os.args.len > 1 {
        return strconv.atoi(os.args[1]) or { default_value }
    }
    return default_value
}
"""
WRITE="""fn emit(data []u8) {
    mut output := os.stdout()
    output.write_string(data.bytestr()) or { panic(err) }
    output.flush()
}
"""

# Published pointer-body update is the optimized scalar baseline. The compact
# form keeps the same arithmetic in ordinary nested V arrays, including all data.
nbody=(UP/'nbody/1.v').read_text()
write('nbody','optimized',nbody,upstream=UP/'nbody/1.v',technique='Published cached scalar body fields, fixed five-body loop and mass-scaled velocity updates.',work=dict(steps='argument',bodies=5,unordered_pairs_per_step=10,energy_before_after=True,approximate_reciprocal=False))
initial=nbody[nbody.index('fn arr_bodies()'):nbody.index('fn main()')]
initial=initial.replace('fn arr_bodies() []&Body','fn arr_bodies() [][]f64').replace('&Body{','[').replace('},','],')
compact_nbody=HEADER+'''import math
const solar_mass = 4.0 * math.pi * math.pi
const days_per_year = 365.24
'''+ARG+initial+'''
fn energy(b [][]f64) f64 {
    mut e := 0.0
    for i in 0 .. b.len {
        e += 0.5 * b[i][6] * (b[i][3] * b[i][3] + b[i][4] * b[i][4] + b[i][5] * b[i][5])
        for j in i + 1 .. b.len {
            dx := b[i][0] - b[j][0]
            dy := b[i][1] - b[j][1]
            dz := b[i][2] - b[j][2]
            e -= b[i][6] * b[j][6] / math.sqrt(dx * dx + dy * dy + dz * dz)
        }
    }
    return e
}
fn main() {
    n := size(1000)
    mut b := arr_bodies()
    for coordinate in 3 .. 6 {
        mut momentum := 0.0
        for body in b {
            momentum += body[coordinate] * body[6]
        }
        b[0][coordinate] = -momentum / solar_mass
    }
    println('${energy(b):.9f}')
    for _ in 0 .. n {
        for i in 0 .. b.len {
            for j in i + 1 .. b.len {
                dx := b[i][0] - b[j][0]
                dy := b[i][1] - b[j][1]
                dz := b[i][2] - b[j][2]
                squared := dx * dx + dy * dy + dz * dz
                magnitude := 0.01 / (squared * math.sqrt(squared))
                displacement := [dx, dy, dz]!
                for coordinate in 0 .. 3 {
                    b[i][coordinate + 3] -= displacement[coordinate] * b[j][6] * magnitude
                    b[j][coordinate + 3] += displacement[coordinate] * b[i][6] * magnitude
                }
            }
        }
        for i in 0 .. b.len {
            for coordinate in 0 .. 3 {
                b[i][coordinate] += 0.01 * b[i][coordinate + 3]
            }
        }
    }
    println('${energy(b):.9f}')
}
'''
write('nbody','compact',compact_nbody,upstream=UP/'nbody/1.v',technique='Ordinary matrix body state and coordinate loops, exact square roots/divisions.',work=dict(steps='argument',bodies=5,unordered_pairs_per_step=10,energy_before_after=True,approximate_reciprocal=False))

mandel_common=HEADER+ARG+WRITE
write('mandelbrot','compact',mandel_common+'''
fn main() {
    n := size(200)
    stride := (n + 7) / 8
    scale := 2.0 / f64(n)
    mut pixels := []u8{len: n * stride}
    for y in 0 .. n {
        ci := f64(y) * scale - 1.0
        for x in 0 .. n {
            cr := f64(x) * scale - 1.5
            mut zr := 0.0
            mut zi := 0.0
            mut inside := true
            for _ in 0 .. 50 {
                next_zr := zr * zr - zi * zi + cr
                zi = 2.0 * zr * zi + ci
                zr = next_zr
                if zr * zr + zi * zi > 4.0 {
                    inside = false
                    break
                }
            }
            if inside {
                pixels[y * stride + x / 8] |= u8(0x80 >> (x % 8))
            }
        }
    }
    print('P4\\n${n} ${n}\\n')
    emit(pixels)
}
''',technique='Scalar escape loop and exact-width PBM including zero padding bits.',work=dict(width='argument',height='argument',iterations=50,complete_pbm=True,round_dimensions=False,md5_only=False))
write('mandelbrot','optimized',mandel_common+'''
struct PixelBlock {
    data []u8
}
@[direct_array_access]
fn pixel_byte(cr [8]f64, ci f64, lanes int) u8 {
    mut zr := [8]f64{}
    mut zi := [8]f64{}
    mut tr := [8]f64{}
    mut ti := [8]f64{}
    for _ in 0 .. 10 {
        for _ in 0 .. 5 {
            for lane in 0 .. 8 {
                zi[lane] = (zr[lane] + zr[lane]) * zi[lane] + ci
                zr[lane] = tr[lane] - ti[lane] + cr[lane]
                tr[lane] = zr[lane] * zr[lane]
                ti[lane] = zi[lane] * zi[lane]
            }
        }
        mut survivors := 0
        for lane in 0 .. lanes {
            if tr[lane] + ti[lane] <= 4.0 {
                survivors++
            }
        }
        if survivors == 0 {
            return 0
        }
    }
    mut bits := u8(0)
    for lane in 0 .. lanes {
        if tr[lane] + ti[lane] <= 4.0 {
            bits |= u8(0x80 >> lane)
        }
    }
    return bits
}
@[direct_array_access]
fn render(n int, first int, last int, x []f64) PixelBlock {
    stride := (n + 7) / 8
    scale := 2.0 / f64(n)
    mut pixels := []u8{len: (last - first) * stride}
    for y in first .. last {
        ci := f64(y) * scale - 1.0
        for block in 0 .. stride {
            lanes := if n - block * 8 < 8 { n - block * 8 } else { 8 }
            mut cr := [8]f64{}
            for lane in 0 .. lanes {
                cr[lane] = x[block * 8 + lane]
            }
            pixels[(y - first) * stride + block] = pixel_byte(cr, ci, lanes)
        }
    }
    return PixelBlock{data: pixels}
}
fn main() {
    n := size(200)
    scale := 2.0 / f64(n)
    mut x := []f64{len: n}
    for i in 0 .. n {
        x[i] = f64(i) * scale - 1.5
    }
    mut jobs := []thread PixelBlock{}
    for worker in 0 .. 8 {
        jobs << spawn render(n, n * worker / 8, n * (worker + 1) / 8, x)
    }
    print('P4\\n${n} ${n}\\n')
    for job in jobs {
        emit(job.wait().data)
    }
}
''',upstream=UP/'mandelbrot/1.v',technique='Eight ordinary V row workers and uniform fixed eight-lane arithmetic with five-iteration escape checks as in the published source; exact PBM/tails replace MD5 and rounded dimensions.',work=dict(width='argument',height='argument',iterations=50,complete_pbm=True,round_dimensions=False,md5_only=False))

spectral_tail='''
fn main() {
    n := size(100)
    mut u := []f64{len: n, init: 1.0}
    mut v := []f64{len: n}
    for _ in 0 .. 10 {
        v = apply(apply(u, false), true)
        u = apply(apply(v, false), true)
    }
    mut uv := 0.0
    mut vv := 0.0
    for i in 0 .. n {
        uv += u[i] * v[i]
        vv += v[i] * v[i]
    }
    println('${math.sqrt(uv / vv):.9f}')
}
'''
write('spectralnorm','compact',HEADER+'import math\n'+ARG+'''
fn apply(u []f64, transpose bool) []f64 {
    mut v := []f64{len: u.len}
    for i in 0 .. u.len {
        mut sum := 0.0
        for j in 0 .. u.len {
            denominator := (i + j) * (i + j + 1) / 2 + (if transpose { j } else { i }) + 1
            sum += u[j] / f64(denominator)
        }
        v[i] = sum
    }
    return v
}
'''+spectral_tail,upstream=UP/'spectral-norm/1.v',technique='One implicit-matrix application with transpose flag and no materialized matrix.',work=dict(size='argument',power_iterations=10,implicit_matrix_products=40,cache_matrix=False))
write('spectralnorm','optimized',HEADER+'import math\n'+ARG+'''
struct VectorBlock {
    data []f64
}
@[direct_array_access]
fn product(u []f64, transpose bool, first int, last int) VectorBlock {
    mut v := []f64{len: last - first}
    extra := if transpose { 1 } else { 0 }
    for i in first .. last {
        mut d := i * (i + 1) / 2 + (if transpose { 0 } else { i }) + 1
        mut a := 0.0
        mut b := 0.0
        mut c := 0.0
        mut e := 0.0
        mut j := 0
        for j + 3 < u.len {
            d1 := d + i + j + 1 + extra
            d2 := d1 + i + j + 2 + extra
            d3 := d2 + i + j + 3 + extra
            a += u[j] / f64(d)
            b += u[j + 1] / f64(d1)
            c += u[j + 2] / f64(d2)
            e += u[j + 3] / f64(d3)
            d = d3 + i + j + 4 + extra
            j += 4
        }
        mut sum := (a + b) + (c + e)
        for j < u.len {
            sum += u[j] / f64(d)
            d += i + j + 1 + extra
            j++
        }
        v[i - first] = sum
    }
    return VectorBlock{data: v}
}
fn apply(u []f64, transpose bool) []f64 {
    mut jobs := []thread VectorBlock{}
    for worker in 0 .. 8 {
        jobs << spawn product(u, transpose, u.len * worker / 8, u.len * (worker + 1) / 8)
    }
    mut v := []f64{cap: u.len}
    for job in jobs {
        v << job.wait().data
    }
    return v
}
'''+spectral_tail,technique='Eight row workers, exact denominator recurrence and four independent accumulation chains with complete scalar tails.',work=dict(size='argument',power_iterations=10,implicit_matrix_products=40,cache_matrix=False))

trees_common=HEADER+ARG+'''
struct Node {
    left &Node = unsafe { nil }
    right &Node = unsafe { nil }
}
fn create(depth int) &Node {
    if depth == 0 {
        return &Node{}
    }
    return &Node{left: create(depth - 1), right: create(depth - 1)}
}
fn check(tree &Node) int {
    if isnil(tree.left) {
        return 1
    }
    return 1 + check(tree.left) + check(tree.right)
}
fn stretch(depth int) {
    tree := create(depth)
    println('stretch tree of depth ${depth}\\t check: ${check(tree)}')
}
'''
write('binarytrees','compact',trees_common+'''
fn main() {
    n := size(10)
    maximum := if n < 6 { 6 } else { n }
    stretch(maximum + 1)
    long_lived := create(maximum)
    for depth := 4; depth <= maximum; depth += 2 {
        count := 1 << (maximum - depth + 4)
        mut sum := 0
        for _ in 0 .. count {
            tree := create(depth)
            sum += check(tree)
        }
        println('${count}\\t trees of depth ${depth}\\t check: ${sum}')
    }
    println('long lived tree of depth ${maximum}\\t check: ${check(long_lived)}')
}
''',upstream=UP/'binarytrees/1.v',technique='Distinct heap Node objects for all internal nodes and leaves; scoped stretch lifetime and retained long-lived root.',work=dict(depth='argument',minimum_max_depth=6,minimum_depth=4,per_node_heap_allocation=True,distinct_leaves=True,long_root_retained=True))
write('binarytrees','optimized',trees_common+'''
struct TreeResult {
    depth int
    count int
    sum int
}
fn trees(depth int, maximum int) TreeResult {
    count := 1 << (maximum - depth + 4)
    mut sum := 0
    for _ in 0 .. count {
        tree := create(depth)
        sum += check(tree)
    }
    return TreeResult{depth: depth, count: count, sum: sum}
}
fn main() {
    n := size(10)
    maximum := if n < 6 { 6 } else { n }
    stretch(maximum + 1)
    long_lived := create(maximum)
    mut depth := 4
    for depth <= maximum {
        mut jobs := []thread TreeResult{}
        for _ in 0 .. 8 {
            if depth <= maximum {
                jobs << spawn trees(depth, maximum)
                depth += 2
            }
        }
        for job in jobs {
            result := job.wait()
            println('${result.count}\\t trees of depth ${result.depth}\\t check: ${result.sum}')
        }
    }
    println('long lived tree of depth ${maximum}\\t check: ${check(long_lived)}')
}
''',technique='At most eight standard V depth workers, ordered result joins; identical actual Node allocations and traversals.',work=dict(depth='argument',minimum_max_depth=6,minimum_depth=4,per_node_heap_allocation=True,distinct_leaves=True,long_root_retained=True))

write('fannkuchredux','compact',(UP/'fannkuch-redux/1.v').read_text(),upstream=UP/'fannkuch-redux/1.v',technique='Published fixed arrays and sequential permutation rotations; every actual prefix reversal and checksum sign.',work=dict(size='argument',canonical_n=12,permutations=479001600,global_rank_parity=True,actual_prefix_flips=True))
write('fannkuchredux','optimized',HEADER+ARG+'''
struct FannResult {
    checksum i64
    maximum int
}
@[direct_array_access]
fn chunk(n int, first i64, last i64, factorial []i64) FannResult {
    mut permutation := [32]int{}
    mut scratch := [32]int{}
    mut digits := [32]int{}
    for i in 0 .. n {
        permutation[i] = i
    }
    mut remaining := first
    for width := n; width > 1; width-- {
        digit := int(remaining / factorial[width - 1])
        remaining %= factorial[width - 1]
        digits[width - 1] = digit
        for _ in 0 .. digit {
            head := permutation[0]
            for i in 0 .. width - 1 {
                permutation[i] = permutation[i + 1]
            }
            permutation[width - 1] = head
        }
    }
    mut checksum := i64(0)
    mut maximum := 0
    for rank := first; rank < last; rank++ {
        for i in 0 .. n {
            scratch[i] = permutation[i]
        }
        mut flips := 0
        for scratch[0] != 0 {
            last_index := scratch[0]
            for i in 0 .. (last_index + 1) / 2 {
                saved := scratch[i]
                scratch[i] = scratch[last_index - i]
                scratch[last_index - i] = saved
            }
            flips++
        }
        if flips > maximum {
            maximum = flips
        }
        checksum += if rank % 2 == 0 { i64(flips) } else { -i64(flips) }
        if rank + 1 < last {
            for width in 2 .. n + 1 {
                head := permutation[0]
                for i in 0 .. width - 1 {
                    permutation[i] = permutation[i + 1]
                }
                permutation[width - 1] = head
                digits[width - 1]++
                if digits[width - 1] < width {
                    break
                }
                digits[width - 1] = 0
            }
        }
    }
    return FannResult{checksum: checksum, maximum: maximum}
}
fn main() {
    n := size(10)
    if n < 2 {
        println('0\\nPfannkuchen(${n}) = 0')
        return
    }
    if n > 20 {
        panic('factorial rank must fit i64')
    }
    mut factorial := []i64{len: n + 1, init: 1}
    for i in 1 .. n + 1 {
        factorial[i] = factorial[i - 1] * i64(i)
    }
    total := factorial[n]
    mut jobs := []thread FannResult{}
    for worker in 0 .. 8 {
        first := total / 8 * i64(worker) + total % 8 * i64(worker) / 8
        last := total / 8 * i64(worker + 1) + total % 8 * i64(worker + 1) / 8
        jobs << spawn chunk(n, first, last, factorial)
    }
    mut checksum := i64(0)
    mut maximum := 0
    for job in jobs {
        result := job.wait()
        checksum += result.checksum
        if result.maximum > maximum {
            maximum = result.maximum
        }
    }
    println('${checksum}\\nPfannkuchen(${n}) = ${maximum}')
}
''',technique='Eight contiguous global-rank ranges, descending-width factoradic initialization, ordinary fixed-array rotations and actual flips.',work=dict(size='argument',canonical_n=12,permutations=479001600,global_rank_parity=True,actual_prefix_flips=True,permutation_cache=False,flip_cache=False))

pi_compact=HEADER+'import math.big\nimport strings\n'+ARG+'''
fn main() {
    n := size(27)
    if n <= 0 {
        return
    }
    one := big.integer_from_int(1)
    three := big.integer_from_int(3)
    four := big.integer_from_int(4)
    ten := big.integer_from_int(10)
    mut q := one
    mut r := big.integer_from_int(0)
    mut t := one
    mut k := 1
    mut printed := 0
    mut line := strings.new_builder(32)
    for printed < n {
        numerator3 := q * three + r
        numerator4 := q * four + r
        digit3 := numerator3 / t
        digit4 := numerator4 / t
        if digit3 == digit4 {
            line.write_byte(u8(48 + digit3.int()))
            printed++
            if printed % 10 == 0 || printed == n {
                for _ in 0 .. (10 - printed % 10) % 10 {
                    line.write_byte(` `)
                }
                line.write_string('\\t:${printed}\\n')
                print(line.str())
                line.go_back_to(0)
            }
            r = (r - digit3 * t) * ten
            q = q * ten
        } else {
            factor := big.integer_from_int(2 * k + 1)
            r = (q * big.integer_from_int(2) + r) * factor
            q = q * big.integer_from_int(k)
            t = t * factor
            k++
        }
    }
}
'''
write('pidigits','compact',pi_compact,technique='Pure V standard math.big LFT with both explicit exact endpoint numerators/divisions for every tested candidate.',work=dict(digits='argument',canonical_n=10000,exact_extracts_per_candidate=2,sequential_spigot=True,output_tail_and_count=True,zero_digits_empty=True),library='V math.big (pure V arbitrary precision)')
pi_optimized=(UP/'pidigits/2.v').read_text().replace('\tdigits_to_print := n','\tif n <= 0 {\n\t\treturn\n\t}\n\n\tdigits_to_print := n')
write('pidigits','optimized',pi_optimized,upstream=UP/'pidigits/2.v',technique='Published endpoint-numerator basis avoids rebuilding3q+r and4q+r each test; both divisions still execute.',work=dict(digits='argument',canonical_n=10000,exact_extracts_per_candidate=2,sequential_spigot=True,output_tail_and_count=True,zero_digits_empty=True),library='V math.big (pure V arbitrary precision)')
pi_optional=(UP/'pidigits/1.v').read_text().replace('\tdigits_to_print := n','\tif n <= 0 {\n\t\treturn\n\t}\n\n\tdigits_to_print := n')
write('pidigits','published-package',pi_optional,upstream=UP/'pidigits/1.v',technique='Optional separately labelled published general biginteger library using the same two-extract endpoint basis; not admitted until its implementation/package pins are reviewed.',work=dict(digits='argument',canonical_n=10000,exact_extracts_per_candidate=2,sequential_spigot=True,output_tail_and_count=True,zero_digits_empty=True),library='hanabi1224.biginteger — public pure-V32-bit-limb library (separate profile)')

alu=(UP/'fasta/1.v').read_text().split("const alu = '",1)[1].split("'.bytes()",1)[0]
fasta_common=HEADER+ARG+WRITE+f"const alu = '{alu}'.bytes()\n"+'''
fn cumulative(probabilities []f64) []f64 {
    mut result := []f64{cap: probabilities.len}
    mut sum := 0.0
    for probability in probabilities {
        sum += probability
        result << sum
    }
    return result
}
'''
fasta_main='''
fn main() {
    n := size(1000)
    iub_symbols := 'acgtBDHKMNRSVWY'.bytes()
    iub_cdf := cumulative([0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02])
    human_symbols := 'acgt'.bytes()
    human_cdf := cumulative([0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])
    println('>ONE Homo sapiens alu')
    repeat_sequence(2 * n)
    println('>TWO IUB ambiguity codes')
    seed := random_sequence(3 * n, iub_symbols, iub_cdf, 42)
    println('>THREE Homo sapiens frequency')
    _ = random_sequence(5 * n, human_symbols, human_cdf, seed)
}
'''
write('fasta','compact',fasta_common+'''
fn repeat_sequence(n int) {
    mut data := []u8{cap: n + n / 60 + 1}
    for i in 0 .. n {
        data << alu[i % alu.len]
        if (i + 1) % 60 == 0 || i + 1 == n {
            data << `\\n`
        }
    }
    emit(data)
}
fn random_sequence(n int, symbols []u8, cdf []f64, seed int) int {
    mut state := seed
    mut data := []u8{cap: n + n / 60 + 1}
    for i in 0 .. n {
        state = (state * 3877 + 29573) % 139968
        random := f64(state) / 139968.0
        mut index := 0
        for cdf[index] <= random {
            index++
        }
        data << symbols[index]
        if (i + 1) % 60 == 0 || i + 1 == n {
            data << `\\n`
        }
    }
    emit(data)
    return state
}
'''+fasta_main,upstream=UP/'fasta/1.v',technique='Explicit ordered binary64 CDFs, complete60-column output, corrected exact-repeat termination and seed carry between random records.',work=dict(size='argument',repeat_bases='2n',iub_bases='3n',human_bases='5n',binary64_cdf=True,integer_scaled_cdf=False,seed_carry=True,complete_output=True))
write('fasta','optimized',fasta_common+'''
fn repeat_sequence(n int) {
    mut buffer := []u8{cap: 65536}
    mut position := 0
    for i in 0 .. n {
        buffer << alu[position]
        position++
        if position == alu.len {
            position = 0
        }
        if (i + 1) % 60 == 0 || i + 1 == n {
            buffer << `\\n`
        }
        if buffer.len >= 65536 {
            emit(buffer)
            buffer.clear()
        }
    }
    emit(buffer)
}
fn random_sequence(n int, symbols []u8, cdf []f64, seed int) int {
    mut state := seed
    mut buffer := []u8{cap: 65536}
    for i in 0 .. n {
        state = (state * 3877 + 29573) % 139968
        random := f64(state) / 139968.0
        mut low := 0
        mut high := cdf.len
        for low < high {
            middle := (low + high) / 2
            if random < cdf[middle] {
                high = middle
            } else {
                low = middle + 1
            }
        }
        buffer << symbols[low]
        if (i + 1) % 60 == 0 || i + 1 == n {
            buffer << `\\n`
        }
        if buffer.len >= 65536 {
            emit(buffer)
            buffer.clear()
        }
    }
    emit(buffer)
    return state
}
'''+fasta_main,technique='Bounded output buffers, wrap counter replacing repeat modulo and binary search over exact binary64 cumulative probabilities.',work=dict(size='argument',repeat_bases='2n',iub_bases='3n',human_bases='5n',binary64_cdf=True,integer_scaled_cdf=False,seed_carry=True,complete_output=True))

rev_common='module main\nimport os\n'+WRITE+'''
fn complement(b u8) u8 {
    symbols := 'ACGTUMRWSYKVHDBN'.bytes()
    values := 'TGCAAKYWSRMBDHVN'.bytes()
    index := symbols.index(b & 0xdf)
    if index < 0 {
        panic('Invalid IUPAC base')
    }
    return values[index]
}
'''
write('revcomp','compact',rev_common+'''
fn record(header string, sequence []u8) {
    if header.len == 0 {
        return
    }
    println(header)
    mut line := []u8{cap: 61}
    for i := sequence.len - 1; i >= 0; i-- {
        line << complement(sequence[i])
        if line.len == 60 || i == 0 {
            line << `\\n`
            emit(line)
            line.clear()
        }
    }
}
fn main() {
    data := os.get_raw_stdin().bytestr()
    mut header := ''
    mut sequence := []u8{}
    for line in data.split_into_lines() {
        if line.starts_with(';') {
            continue
        } else if line.starts_with('>') {
            record(header, sequence)
            header = line
            sequence.clear()
        } else {
            sequence << line.bytes()
        }
    }
    record(header, sequence)
}
''',technique='Complete FASTA records, case-insensitive IUPAC complement and60-column output.',work=dict(stdin_fasta=True,all_records=True,iupac=True,wrap_columns=60,complete_output=True))
write('revcomp','optimized','module main\nimport os\n'+WRITE+'''
fn record(header []u8, mut sequence []u8, table [256]u8) {
    if header.len == 0 {
        return
    }
    emit(header)
    mut left := 0
    mut right := sequence.len - 1
    for left <= right {
        a := table[sequence[left]]
        b := table[sequence[right]]
        if a == 0 || b == 0 {
            panic('Invalid IUPAC base')
        }
        sequence[left] = b
        sequence[right] = a
        left++
        right--
    }
    mut output := []u8{cap: sequence.len + sequence.len / 60 + 1}
    for start := 0; start < sequence.len; start += 60 {
        end := if start + 60 < sequence.len { start + 60 } else { sequence.len }
        output << sequence[start .. end]
        output << `\\n`
    }
    emit(output)
}
fn main() {
    symbols := 'ACGTUMRWSYKVHDBN'.bytes()
    values := 'TGCAAKYWSRMBDHVN'.bytes()
    mut table := [256]u8{}
    for i, symbol in symbols {
        table[symbol] = values[i]
        table[symbol + 32] = values[i]
    }
    data := os.get_raw_stdin()
    mut header := []u8{}
    mut sequence := []u8{}
    mut at_line_start := true
    mut position := 0
    for position < data.len {
        b := data[position]
        if at_line_start && b == `;` {
            for position < data.len && data[position] != `\\n` {
                position++
            }
            if position < data.len {
                position++
            }
            at_line_start = true
            continue
        } else if at_line_start && b == `>` {
            record(header, mut sequence, table)
            sequence.clear()
            header.clear()
            for position < data.len && data[position] != `\\n` {
                if data[position] != `\\r` {
                    header << data[position]
                }
                position++
            }
            header << `\\n`
            if position < data.len {
                position++
            }
            at_line_start = true
            continue
        } else if b != `\\n` && b != `\\r` {
            sequence << b
        }
        at_line_start = b == `\\n`
        position++
    }
    record(header, mut sequence, table)
}
''',technique='Byte-oriented FASTA parser, general256-byte IUPAC table and in-place reversal, bulk wrapped output.',work=dict(stdin_fasta=True,all_records=True,iupac=True,wrap_columns=60,complete_output=True))

knuc_common='module main\nimport os\n'+'''
const lengths = [1,2,3,4,6,12,18]
const queries = ['GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT']
struct Entry {
    word string
    count u64
}
fn third_record() []u8 {
    input := os.get_raw_stdin().bytestr()
    mut selected := false
    mut sequence := []u8{}
    for line in input.split_into_lines() {
        if line.starts_with(';') {
            continue
        } else if line.starts_with('>') {
            if selected {
                break
            }
            selected = line.starts_with('>THREE ') || line.starts_with('>THREE\\t') || line == '>THREE'
        } else if selected {
            sequence << line.to_upper().bytes()
        }
    }
    return sequence
}
fn compare_entries(a &Entry, b &Entry) int {
    if a.count == b.count {
        return if a.word < b.word { -1 } else if a.word > b.word { 1 } else { 0 }
    }
    return if a.count > b.count { -1 } else { 1 }
}
fn frequencies(mut rows []Entry, denominator int) {
    rows.sort_with_compare(compare_entries)
    for row in rows {
        println('${row.word} ${100.0 * f64(row.count) / f64(denominator):.3f}')
    }
    println('')
}
'''
write('knucleotide','compact',knuc_common+'''
fn counts(sequence string, k int) map[string]u64 {
    mut result := map[string]u64{}
    if sequence.len >= k {
        for i in 0 .. sequence.len - k + 1 {
            word := sequence[i .. i + k]
            result[word] = result[word] + 1
        }
    }
    return result
}
fn main() {
    sequence := third_record().bytestr()
    mut tables := []map[string]u64{}
    for k in lengths {
        tables << counts(sequence, k)
    }
    for index in 0 .. 2 {
        mut rows := []Entry{}
        for word, count in tables[index] {
            rows << Entry{word: word, count: count}
        }
        frequencies(mut rows, sequence.len - lengths[index] + 1)
    }
    for index, word in queries {
        println('${tables[index + 2][word]}\\t${word}')
    }
}
''',technique='Seven independent growable string-key maps, visiting every overlapping window for every required length.',work=dict(target_record='THREE',lengths=[1,2,3,4,6,12,18],all_windows=True,growable_maps=7,small_tables_are_maps=True,query_shortcuts=False))
write('knucleotide','optimized',knuc_common+'''
struct CountResult {
    k int
    counts map[u64]u64
}
fn code(b u8) u64 {
    return match b {
        `A` { u64(0) }
        `C` { u64(1) }
        `G` { u64(2) }
        `T` { u64(3) }
        else {
            panic('Invalid nucleotide')
        }
    }
}
@[direct_array_access]
fn packed(sequence []u8, k int) CountResult {
    mask := (u64(1) << (2 * k)) - 1
    mut key := u64(0)
    mut result := map[u64]u64{}
    for i, b in sequence {
        key = ((key << 2) | code(b)) & mask
        if i + 1 >= k {
            result[key] = result[key] + 1
        }
    }
    return CountResult{k: k, counts: result}
}
fn decode(key u64, k int) string {
    symbols := 'ACGT'.bytes()
    mut remaining := key
    mut word := []u8{len: k}
    for i := k - 1; i >= 0; i-- {
        word[i] = symbols[int(remaining & 3)]
        remaining >>= 2
    }
    return word.bytestr()
}
fn main() {
    sequence := third_record()
    mut jobs := []thread CountResult{}
    for k in lengths {
        jobs << spawn packed(sequence, k)
    }
    mut tables := []CountResult{}
    for job in jobs {
        tables << job.wait()
    }
    for index in 0 .. 2 {
        mut rows := []Entry{}
        for key, count in tables[index].counts {
            rows << Entry{word: decode(key, lengths[index]), count: count}
        }
        frequencies(mut rows, sequence.len - lengths[index] + 1)
    }
    for index, word in queries {
        mut key := u64(0)
        for b in word.bytes() {
            key = (key << 2) | code(b)
        }
        println('${tables[index + 2].counts[key]}\\t${word}')
    }
}
''',technique='Seven ordinary map workers with rolling2-bit keys and exact all-window growable maps, including lengths1/2; decode only output keys.',work=dict(target_record='THREE',lengths=[1,2,3,4,6,12,18],all_windows=True,growable_maps=7,small_tables_are_maps=True,query_shortcuts=False,maximum_compute_workers=7))

patterns="""const patterns = ['agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]','a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct','agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct','agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct','agggtaa[cgt]|[acg]ttaccct']
const replacements = ['tHa[Nt]','(aND)|(caN)|(Ha[DS])|(WaS)','(a[NSt])|(BY)','<[^>]*>','[|][^|][^|]*[|]']
const replacement_values = ['<4>','<3>','<2>','|','-']
"""
regex_common='module main\nimport os\nimport regex\n'+patterns+'''
fn count(sequence string, pattern string) int {
    mut alternatives := []string{}
    for part in pattern.split('|') {
        alternatives << '(${part})'
    }
    mut expression := regex.regex_opt(alternatives.join('|')) or { panic(err) }
    return expression.find_all(sequence).len / 2
}
fn replaced(sequence string) string {
    mut result := sequence
    for i, pattern in replacements {
        mut expression := regex.regex_opt(pattern) or { panic(err) }
        result = expression.replace(result, replacement_values[i])
    }
    return result
}
'''
write('regexredux','compact',regex_common+'''
fn main() {
    input := os.get_raw_stdin().bytestr()
    mut cleanup := regex.regex_opt('(>.*\\n)|(\\n)') or { panic(err) }
    sequence := cleanup.replace(input, '')
    for pattern in patterns {
        println('${pattern} ${count(sequence, pattern)}')
    }
    final := replaced(sequence)
    println('\\n${input.len}\\n${sequence.len}\\n${final.len}')
}
''',upstream=UP/'regex-redux/1.v',technique='Standard V regex implementation, stdin and explicit ordered replacement arrays replacing an implicit map order.',work=dict(stdin_fasta=True,count_patterns=9,ordered_replacements=5,complete_input=True,full_output=True))
write('regexredux','optimized',regex_common+'''
fn main() {
    input := os.get_raw_stdin().bytestr()
    mut cleanup := regex.regex_opt('(>.*\\n)|(\\n)') or { panic(err) }
    sequence := cleanup.replace(input, '')
    mut jobs := []thread int{}
    for i in 0 .. 8 {
        jobs << spawn count(sequence, patterns[i])
    }
    mut counts := []int{}
    for job in jobs {
        counts << job.wait()
    }
    counts << count(sequence, patterns[8])
    for i, pattern in patterns {
        println('${pattern} ${counts[i]}')
    }
    final := replaced(sequence)
    println('\\n${input.len}\\n${sequence.len}\\n${final.len}')
}
''',technique='Eight independent standard regex count workers plus ninth after joins; identical generic regex cleanup and all five sequential ordered replacements.',work=dict(stdin_fasta=True,count_patterns=9,ordered_replacements=5,complete_input=True,full_output=True))

(HERE/'profiles.json').write_text(json.dumps(dict(status='source-drafts-unbuilt-unqualified',report_only=True,chart_admission=False,v_release='0.5.2',v_source_ref='7647ce1c6fad63b5578bc07883139906de74b2f8',canonical_normalization='Pending root-owned common rule: count written headers/closures; split independent executable bindings; data and opening-only delimiters remain structural.',profiles=profiles),indent=2)+'\n')
print('Authored V drafts and profile plan only; no V build, runtime, hashes, probes or timings.')
