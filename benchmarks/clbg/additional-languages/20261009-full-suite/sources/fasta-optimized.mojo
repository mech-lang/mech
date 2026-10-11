from std.sys import argv
from max.algorithm.backend.cpu.parallelize import parallelize
from common import write_bytes
comptime ALU = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

def random_record(n: Int, mut seed: UInt32, imm symbols: String, var probabilities: List[Float64]) -> List[UInt8]:
    var sum: Float64 = 0
    for ref p in probabilities:
        sum += p
        p = sum
    var states = List[UInt32](unsafe_uninit_length=n)
    var rng = states.unsafe_ptr()
    for i in range(n):
        seed = (seed * 3877 + 29573) % 139968
        rng[i] = seed
    var size = n + (n + 59) // 60
    var bytes = List[UInt8](unsafe_uninit_length=size)
    var out = bytes.unsafe_ptr()
    var alphabet = symbols.as_bytes().unsafe_ptr()
    var cdf = probabilities.unsafe_ptr()
    var choices = len(probabilities)
    def worker(task: Int) {var}:
        for j in range(task * size // 32, (task + 1) * size // 32):
            var i = j // 61 * 60 + j % 61
            if j % 61 == 60 or i >= n:
                out[j] = 10
            else:
                var p = Float64(rng[i]) / 139968.0
                var k = 0
                while k + 1 < choices and p >= cdf[k]:
                    k += 1
                out[j] = alphabet[k]
    parallelize(worker, 32, 8)
    return bytes^

def main() raises:
    var n = Int(argv()[1])
    var out = List[UInt8](capacity=10 * n + (10 * n + 59) // 60 + 100)
    out.extend(">ONE Homo sapiens alu\n".as_bytes())
    var size = 2 * n + (2 * n + 59) // 60
    var repeated = List[UInt8](unsafe_uninit_length=size)
    var dst = repeated.unsafe_ptr()
    var alu = ALU.as_bytes().unsafe_ptr()
    def worker(task: Int) {var}:
        for j in range(task * size // 32, (task + 1) * size // 32):
            var i = j // 61 * 60 + j % 61
            dst[j] = 10 if j % 61 == 60 or i >= 2 * n else alu[i % 287]
    parallelize(worker, 32, 8)
    out.extend(repeated^)
    out.extend(">TWO IUB ambiguity codes\n".as_bytes())
    var seed: UInt32 = 42
    out.extend(random_record(3 * n, seed, "acgtBDHKMNRSVWY", [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02]))
    out.extend(">THREE Homo sapiens frequency\n".as_bytes())
    out.extend(random_record(5 * n, seed, "acgt", [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008]))
    write_bytes(out)
