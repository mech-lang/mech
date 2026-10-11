"""Measured-language application sources, with ordinary arrays/maps and FFI APIs."""
NBODY='''# Locally authored full binary64 pairwise n-body workload.
import sys
from math import sqrt
mass = 4.0 * 3.141592653589793 * 3.141592653589793
year = 365.24
bodies = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, mass],
    [4.84143144246472090, -1.16032004402742839, -0.103622044471123109, 0.00166007664274403694, 0.00769901118419740425, -0.0000690460016972063023, 0.000954791938424326609],
    [8.34336671824457987, 4.12479856412430479, -0.403523417114321381, -0.00276742510726862411, 0.00499852801234917238, 0.0000230417297573763929, 0.000285885980666130812],
    [12.8943695621391310, -15.1111514016986312, -0.223307578892655734, 0.00296460137564761618, 0.00237847173959480950, -0.0000296589568540253756, 0.0000436624404335156298],
    [15.3796971148509165, -25.9193146099879641, 0.179258772950371181, 0.00268067772490389322, 0.00162824170038242295, -0.0000951592254519715870, 0.0000515138902046611451]]
for b in bodies[1:]:
    b[6] *= mass
    for axis in range(3, 6):
        b[axis] *= year
for axis in range(3, 6):
    bodies[0][axis] = -sum(b[axis] * b[6] for b in bodies) / mass
pairs = [(a, b) for i, a in enumerate(bodies) for b in bodies[i+1:]]
def energy():
    kinetic = sum(0.5 * b[6] * sum(v*v for v in b[3:6]) for b in bodies)
    return kinetic - sum(a[6] * b[6] / sqrt(sum((a[i]-b[i])**2 for i in range(3))) for a, b in pairs)
print("%.9f" % energy())
for step in range(int(sys.argv[1])):
    for a, b in pairs:
        delta = [a[i]-b[i] for i in range(3)]
        square = sum(d*d for d in delta)
        magnitude = 0.01 / (square * sqrt(square))
        for axis in range(3):
            a[axis+3] -= delta[axis] * b[6] * magnitude
            b[axis+3] += delta[axis] * a[6] * magnitude
    for b in bodies:
        for axis in range(3):
            b[axis] += 0.01 * b[axis+3]
print("%.9f" % energy())
'''

MANDELBROT='''# Scalar binary64 pixels, strict radius test, complete padded PBM rows.
import sys
n = int(sys.argv[1])
write = sys.stdout.buffer.write
write(("P4\\n%d %d\\n" % (n, n)).encode())
real = [2.0*x/n - 1.5 for x in range(n)]
for y in range(n):
    ci = 2.0*y/n - 1.0
    row = bytearray((n+7)//8)
    for x in range(n):
        zr = 0.0
        zi = 0.0
        inside = 1
        for iteration in range(50):
            next_real = zr*zr - zi*zi + real[x]
            zi = 2.0*zr*zi + ci
            zr = next_real
            if zr*zr + zi*zi > 4.0:
                inside = 0
                break
        row[x//8] |= inside << (7-x%8)
    write(row)
'''

SPECTRAL='''# Inline complete implicit-matrix products; no precomputed workload kernel.
import sys
from math import sqrt
def multiply(x, transpose):
    n = len(x)
    out = [0.0]*n
    for i in range(n):
        total = 0.0
        for j in range(n):
            s = i+j
            denominator = s*(s+1)//2 + (j if transpose else i) + 1
            total += x[j] / denominator
        out[i] = total
    return out
def step(x):
    return multiply(multiply(x, False), True)
n = int(sys.argv[1])
u = [1.0]*n
v = [0.0]*n
for iteration in range(10):
    v = step(u)
    u = step(v)
print("%.9f" % sqrt(sum(a*b for a,b in zip(u,v))/sum(b*b for b in v)))
'''

FASTA='''# Buffered output, binary64 cumulative probabilities, every original LCG step.
import sys
n = int(sys.argv[1])
write = sys.stdout.buffer.write
alu = b"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
seed = 42
def repeat(count):
    offset = 0
    pending = bytearray()
    for start in range(0, count, 60):
        for i in range(min(60, count-start)):
            pending.append(alu[offset])
            offset = (offset+1) % len(alu)
        pending.append(10)
        if len(pending) >= 65536:
            write(pending)
            pending = bytearray()
    write(pending)
def random(count, symbols, probabilities):
    global seed
    cumulative = []
    total = 0.0
    for probability in probabilities:
        total += probability
        cumulative.append(total)
    pending = bytearray()
    for start in range(0, count, 60):
        for i in range(min(60, count-start)):
            seed = (seed*3877+29573) % 139968
            r = float(seed) / 139968.0
            index = 0
            while index+1 < len(cumulative) and r >= cumulative[index]:
                index += 1
            pending.append(symbols[index])
        pending.append(10)
        if len(pending) >= 65536:
            write(pending)
            pending = bytearray()
    write(pending)
write(b">ONE Homo sapiens alu\\n")
repeat(2*n)
write(b">TWO IUB ambiguity codes\\n")
random(3*n, b"acgtBDHKMNRSVWY", [0.27,0.12,0.12,0.27]+[0.02]*11)
write(b">THREE Homo sapiens frequency\\n")
random(5*n, b"acgt", [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])
'''

K='''# Complete seven growable hash maps, collision-free full 36-bit rolling keys.
import sys
lines = []
for line in sys.stdin:
    if line.startswith(">THREE"):
        break
for line in sys.stdin:
    if line.startswith(">"):
        break
    lines.append(line.strip().upper())
dna = "".join(lines)
codes = bytes((ord(c)&6)>>1 for c in dna)
def packed(text):
    key = 0
    for c in text:
        key = (key<<2) | ((ord(c)&6)>>1)
    return key
def decode(key, width):
    return "".join("ACTG"[(key>>(2*i))&3] for i in range(width-1,-1,-1))
def counts(width):
    result = {}
    mask = (1<<(2*width))-1
    key = 0
    for i, code in enumerate(codes):
        key = ((key<<2)|code)&mask
        if i >= width-1:
            result[key] = result.get(key,0)+1
    return result
for width in [1,2]:
    table = counts(width)
    for key, count in sorted(table.items(), key=lambda p: (-p[1],decode(p[0],width))):
        print("%s %.3f" % (decode(key,width),100.0*count/(len(codes)-width+1)))
    print()
for query in ["GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"]:
    print("%d\\t%s" % (counts(len(query)).get(packed(query),0),query))
'''

REVCOMP='''# Full IUPAC complement/reversal and every record, using ordinary byte arrays.
import sys
table = bytearray(range(256))
for a,b in zip(b"ACBDGHKMNSRUTWVYacbdghkmnsrutwvy", b"TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"):
    table[a] = b
def show(header, sequence):
    sys.stdout.buffer.write(header.rstrip()+b"\\n")
    for start in range(len(sequence)-1,-1,-60):
        end = max(-1,start-60)
        row = bytes(table[sequence[i]] for i in range(start,end,-1))
        sys.stdout.buffer.write(row+b"\\n")
header = None
sequence = bytearray()
for line in sys.stdin.buffer:
    if line.startswith(b">"):
        if header is not None:
            show(header,sequence)
        header = line
        sequence = bytearray()
    else:
        sequence.extend(line.strip())
if header is not None:
    show(header,sequence)
'''

FANN='''# Every factorial permutation, checksum and flip maximum; built-in list slices.
import sys
n = int(sys.argv[1])
permutation = list(range(n))
count = [0]*n
r = n
index = 0
checksum = 0
maximum = 0
while True:
    while r > 1:
        count[r-1] = r
        r -= 1
    work = permutation[:]
    flips = 0
    while work[0]:
        k = work[0]
        work[:k+1] = work[k::-1]
        flips += 1
    maximum = max(maximum, flips)
    checksum += flips if index%2 == 0 else -flips
    while r < n:
        first = permutation[0]
        permutation[:r] = permutation[1:r+1]
        permutation[r] = first
        count[r] -= 1
        if count[r]:
            break
        r += 1
    if r == n:
        break
    index += 1
print(checksum)
print("Pfannkuchen(%d) = %d" % (n, maximum))
'''

PI='''# Unbounded integer Gibbons LFT; both extract(3) and extract(4) each attempt.
import sys
limit = int(sys.argv[1])
q = 1
r = 0
t = 1
k = 1
count = 0
line = ""
while count < limit:
    a = (3*q+r)//t
    b = (4*q+r)//t
    if a == b:
        line += str(a)
        count += 1
        if count%10 == 0 or count == limit:
            print(line+" "*(10-len(line))+"\\t:"+str(count))
            line = ""
        r = 10*(r-a*t)
        q *= 10
    else:
        r = (2*q+r)*(2*k+1)
        q *= k
        t *= 2*k+1
        k += 1
'''

REGEX='''# Ordinary PCRE2 FFI only; pattern selection, counting and ordered passes in Python.
import sys
import ffi
import uctypes
import struct
lib = ffi.open("/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib")
compile = lib.func("p", "pcre2_compile_8", "sLippp")
jit = lib.func("i", "pcre2_jit_compile_8", "pi")
new_match = lib.func("p", "pcre2_match_data_create_from_pattern_8", "pp")
match = lib.func("i", "pcre2_match_8", "psLLipp")
vector = lib.func("p", "pcre2_get_ovector_pointer_8", "p")
substitute = lib.func("i", "pcre2_substitute_8", "psLLippsLpp")
free_code = lib.func("v", "pcre2_code_free_8", "p")
free_match = lib.func("v", "pcre2_match_data_free_8", "p")
class Regex:
    def __init__(self, pattern):
        error = bytearray(4)
        offset = bytearray(8)
        self.code = compile(pattern, len(pattern), 0, error, offset, 0)
        if not self.code:
            raise ValueError("PCRE2 compile error")
        # JIT_LINE
        self.data = new_match(self.code, 0)
    def count(self, text):
        offset = 0
        total = 0
        while match(self.code, text, len(text), offset, 0, self.data, 0) >= 0:
            start, end = struct.unpack("QQ", uctypes.bytearray_at(vector(self.data),16))
            total += 1
            offset = end if end > offset else offset+1
            if offset > len(text):
                break
        return total
    def replace(self, text, replacement):
        capacity = 2*len(text)+64
        while True:
            result = bytearray(capacity)
            size = bytearray(struct.pack("Q",capacity))
            status = substitute(self.code,text,len(text),0,0x1100,self.data,0,replacement,len(replacement),result,size)
            required = struct.unpack("Q",size)[0]
            if status == -48 and required > capacity:
                capacity = required
                continue
            if status < 0:
                raise ValueError("PCRE2 replacement error "+str(status))
            return bytes(result[:required])
    def close(self):
        free_match(self.data)
        free_code(self.code)
def replace(pattern, text, replacement):
    regex = Regex(pattern)
    result = regex.replace(text,replacement)
    regex.close()
    return result
source = sys.stdin.buffer.read()
text = replace(">.*\\n|\\n",source,b"")
patterns = ["agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]","a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct","agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct","agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct","agggtaa[cgt]|[acg]ttaccct"]
for pattern in patterns:
    regex = Regex(pattern)
    print(pattern,regex.count(text))
    regex.close()
result = text
for pattern, replacement in [("tHa[Nt]",b"<4>"),("aND|caN|Ha[DS]|WaS",b"<3>"),("a[NSt]|BY",b"<2>"),("<[^>]*>",b"|"),("\\\\|[^|][^|]*\\\\|",b"-")]:
    result = replace(pattern,result,replacement)
print("\\n%d\\n%d\\n%d" % (len(source),len(text),len(result)))
'''

TREE='''# Equal-size distinct two-slot records in a growable standard array arena.
import sys
from array import array
class Tree:
    def __init__(self, depth):
        self.nodes = array("i")
        self.root = self.build(depth)
    def build(self, depth):
        left = -1
        right = -1
        if depth:
            left = self.build(depth-1)
            right = self.build(depth-1)
        index = len(self.nodes)
        self.nodes.append(left)
        self.nodes.append(right)
        return index
    def check(self, index=None):
        if index is None:
            index = self.root
        left = self.nodes[index]
        if left == -1:
            return 1
        return 1+self.check(left)+self.check(self.nodes[index+1])
maximum = max(6,int(sys.argv[1]))
stretch = maximum+1
print("stretch tree of depth %d\\t check: %d" % (stretch,Tree(stretch).check()))
long_lived = Tree(maximum)
for depth in range(4,maximum+1,2):
    count = 1<<(maximum-depth+4)
    total = 0
    for iteration in range(count):
        total += Tree(depth).check()
    print("%d\\t trees of depth %d\\t check: %d" % (count,depth,total))
print("long lived tree of depth %d\\t check: %d" % (maximum,long_lived.check()))
'''

PI_GMP='''# Ordinary general GMP arithmetic; LFT control and both extractions in Python.
import sys
import ffi
lib = ffi.open("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib")
init = lib.func("v","__gmpz_init_set_si","pl")
multiply = lib.func("v","__gmpz_mul_ui","ppL")
add = lib.func("v","__gmpz_add","ppp")
subtract = lib.func("v","__gmpz_sub","ppp")
divide = lib.func("v","__gmpz_fdiv_q","ppp")
compare = lib.func("i","__gmpz_cmp","pp")
value = lib.func("l","__gmpz_get_si","p")
clear = lib.func("v","__gmpz_clear","p")
def integer(value):
    result = bytearray(16)
    init(result,value)
    return result
q = integer(1)
r = integer(0)
t = integer(1)
a = integer(0)
b = integer(0)
temporary = integer(0)
k = 1
count = 0
limit = int(sys.argv[1])
line = ""
while count < limit:
    multiply(temporary,q,3)
    add(temporary,temporary,r)
    divide(a,temporary,t)
    multiply(temporary,q,4)
    add(temporary,temporary,r)
    divide(b,temporary,t)
    if compare(a,b) == 0:
        digit = value(a)
        line += str(digit)
        count += 1
        if count%10 == 0 or count == limit:
            print(line+" "*(10-len(line))+"\\t:"+str(count))
            line = ""
        multiply(temporary,t,digit)
        subtract(r,r,temporary)
        multiply(r,r,10)
        multiply(q,q,10)
    else:
        multiply(temporary,q,2)
        add(r,temporary,r)
        multiply(r,r,2*k+1)
        multiply(q,q,k)
        multiply(t,t,2*k+1)
        k += 1
for number in [q,r,t,a,b,temporary]:
    clear(number)
'''

REVCOMP_FAST='''# Ordinary mutable reversal, generic complement table, buffered wrapped output.
import sys
table = bytearray(range(256))
for a,b in zip(b"ACBDGHKMNSRUTWVYacbdghkmnsrutwvy", b"TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"):
    table[a] = b
write = sys.stdout.buffer.write
def show(header, sequence):
    write(header.rstrip()+b"\\n")
    low = 0
    high = len(sequence)-1
    while low <= high:
        complement = table[sequence[low]]
        sequence[low] = table[sequence[high]]
        sequence[high] = complement
        low += 1
        high -= 1
    pending = bytearray()
    for start in range(0,len(sequence),60):
        pending.extend(sequence[start:start+60])
        pending.append(10)
        if len(pending) >= 65536:
            write(pending)
            pending = bytearray()
    write(pending)
header = None
sequence = bytearray()
for line in sys.stdin.buffer:
    if line.startswith(b">"):
        if header is not None:
            show(header,sequence)
        header = line
        sequence = bytearray()
    else:
        sequence.extend(line.strip())
if header is not None:
    show(header,sequence)
'''
