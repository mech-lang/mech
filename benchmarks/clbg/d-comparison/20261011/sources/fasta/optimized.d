// Buffered binary64 FASTA with statically known probability-table widths.
import std;
uint seed = 42;
char[1 << 20] buffer;
size_t used;
void emit(const(char)[] text) {
    if (used + text.length > buffer.length) {
        stdout.rawWrite(buffer[0 .. used]);
        used = 0;
    }
    buffer[used .. used+text.length] = text;
    used += text.length;
}
void randomSequence(size_t N)(string letters, double[N] probabilities, size_t n) {
    double sum = 0.0;
    foreach (ref p; probabilities)
        p = sum += p;
    char[61] line;
    while (n) {
        auto length = min(n,60);
        foreach (i; 0 .. length) {
            seed = (seed * 3877 + 29573) % 139968;
            auto r = cast(double)seed / 139968;
            size_t index;
            static foreach (j; 0 .. N)
                index += probabilities[j] <= r;
            line[i] = letters[index];
        }
        line[length] = '\n';
        emit(line[0 .. length+1]);
        n -= length;
    }
}
void main(string[] args) {
    auto n = args[1].to!size_t;
    immutable alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
    emit(">ONE Homo sapiens alu\n");
    char[61] line;
    size_t offset;
    foreach (start; iota(0, n*2,60)) {
        auto length = min(n*2-start,60);
        foreach (i; 0 .. length) {
            line[i] = alu[offset];
            if (++offset == alu.length)
                offset = 0;
        }
        line[length] = '\n';
        emit(line[0 .. length+1]);
    }
    emit(">TWO IUB ambiguity codes\n");
    randomSequence!15("acgtBDHKMNRSVWY", [0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02],n*3);
    emit(">THREE Homo sapiens frequency\n");
    randomSequence!4("acgt", [0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008],n*5);
    stdout.rawWrite(buffer[0 .. used]);
}
