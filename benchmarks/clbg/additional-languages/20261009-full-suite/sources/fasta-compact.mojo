from std.sys import argv
from common import wrap, write_bytes
comptime ALU = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

def record(n: Int, mut seed: Int, imm symbols: String, var probabilities: List[Float64]) -> List[UInt8]:
    var cdf: Float64 = 0
    for ref p in probabilities:
        cdf += p
        p = cdf
    var out = List[UInt8](capacity=n)
    var bytes = symbols.as_bytes()
    for i in range(n):
        var k = i % len(bytes)
        if len(probabilities):
            seed = (seed * 3877 + 29573) % 139968
            var p = Float64(seed) / 139968.0
            k = 0
            while k + 1 < len(probabilities) and p >= probabilities[k]:
                k += 1
        out.append(bytes[k])
    return wrap(60, out)

def main() raises:
    var n = Int(argv()[1])
    var seed = 42
    var out = List[UInt8]()
    out.extend(">ONE Homo sapiens alu\n".as_bytes())
    out.extend(record(2 * n, seed, ALU, List[Float64]()))
    out.extend(">TWO IUB ambiguity codes\n".as_bytes())
    out.extend(record(3 * n, seed, "acgtBDHKMNRSVWY", [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02]))
    out.extend(">THREE Homo sapiens frequency\n".as_bytes())
    out.extend(record(5 * n, seed, "acgt", [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008]))
    write_bytes(out)
