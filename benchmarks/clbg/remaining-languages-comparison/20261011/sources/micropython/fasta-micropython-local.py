# Buffered output, binary64 cumulative probabilities, every original LCG step.
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
write(b">ONE Homo sapiens alu\n")
repeat(2*n)
write(b">TWO IUB ambiguity codes\n")
random(3*n, b"acgtBDHKMNRSVWY", [0.27,0.12,0.12,0.27]+[0.02]*11)
write(b">THREE Homo sapiens frequency\n")
random(5*n, b"acgt", [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])
