// Compact binary64 adaptation of the published D FASTA implementation.
import std;
uint seed = 42;
immutable alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
void generate(string letters, double[] probabilities, size_t n) {
    double sum = 0.0;
    foreach (ref p; probabilities)
        p = sum += p;
    while (n) {
        auto line = new char[min(n,60)];
        foreach (i; 0 .. line.length) {
            seed = (seed * 3877 + 29573) % 139968;
            auto r = cast(double)seed / 139968;
            line[i] = letters[probabilities.countUntil!(p => p > r)];
        }
        writeln(line);
        n -= line.length;
    }
}
void main(string[] args) {
    auto n = args[1].to!size_t;
    writeln(">ONE Homo sapiens alu");
    foreach (offset; iota(0, n*2, 60))
        writeln(iota(offset, min(offset+60,n*2)).map!(i => alu[i % alu.length]).array);
    writeln(">TWO IUB ambiguity codes");
    generate("acgtBDHKMNRSVWY", [0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02], n*3);
    writeln(">THREE Homo sapiens frequency");
    generate("acgt", [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008], n*5);
}
