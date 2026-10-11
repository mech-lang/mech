from std.sys import argv
from common import write_bytes
comptime ALU = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

def header(mut out: List[UInt8], imm text: String):
    for c in text.as_bytes():
        out.append(c)

def random_record(mut out: List[UInt8], n: Int, seed: Int,
                 imm symbols: String, mut probabilities: List[Float64]) -> Int:
    var sum: Float64 = 0
    for i in range(len(probabilities)):
        sum += probabilities[i]
        probabilities[i] = sum
    var s = seed
    var bytes = symbols.as_bytes()
    for i in range(n):
        s = (s * 3877 + 29573) % 139968
        var p = Float64(s) / 139968.0
        var k = 0
        while k + 1 < len(probabilities) and p >= probabilities[k]:
            k += 1
        out.append(bytes[k])
        if i % 60 == 59:
            out.append(10)
    if n % 60 != 0:
        out.append(10)
    return s

def main() raises:
    var n = Int(argv()[1])
    var out = List[UInt8](capacity=10 * n + (10 * n + 59) // 60 + 100)
    header(out, ">ONE Homo sapiens alu\n")
    var alu = ALU.as_bytes()
    for i in range(2 * n):
        out.append(alu[i % len(alu)])
        if i % 60 == 59:
            out.append(10)
    if 2 * n % 60 != 0:
        out.append(10)
    header(out, ">TWO IUB ambiguity codes\n")
    var iub: List[Float64] = [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02]
    var seed = random_record(out, 3 * n, 42, "acgtBDHKMNRSVWY", iub)
    header(out, ">THREE Homo sapiens frequency\n")
    var homo: List[Float64] = [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008]
    _ = random_record(out, 5 * n, seed, "acgt", homo)
    write_bytes(out)
