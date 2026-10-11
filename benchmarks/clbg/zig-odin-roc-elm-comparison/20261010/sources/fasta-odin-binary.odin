package main
import "core:bufio"
import "core:os"
import "core:strconv"
ALU :: "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTC" +
    "AGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCG" +
    "TGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGG" +
    "AGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
random_fasta :: proc(out: ^bufio.Writer, length: int, letters: string, probabilities: []f64, seed: ^u32) {
    cumulative: [15]f64
    total := 0.0
    for probability, i in probabilities {
        total += probability
        cumulative[i] = total
    }
    line: [61]u8
    remaining := length
    for remaining > 0 {
        take := min(60, remaining)
        for i in 0..<take {
            seed^ = (seed^*3877 + 29573)%139968
            random := f64(seed^)/139968.0
            choice := 0
            low := 0
            high := len(probabilities)-1
            for low < high {
                middle := (low+high)/2
                if random < cumulative[middle] { high = middle } else { low = middle+1 }
            }
            choice = low
            line[i] = letters[choice]
        }
        line[take] = '\n'
        _, err := bufio.writer_write(out, line[:take+1])
        assert(err == nil)
        remaining -= take
    }
}
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 1000
    out: bufio.Writer
    bufio.writer_init(&out, os.to_stream(os.stdout), 65536)
    defer bufio.writer_destroy(&out)
    _, _ = bufio.writer_write_string(&out, ">ONE Homo sapiens alu\n")
    alu := ALU
    line: [61]u8
    offset := 0
    remaining := 2*n
    for remaining > 0 {
        take := min(60, remaining)
        for i in 0..<take { line[i] = alu[(offset+i)%len(alu)] }
        line[take] = '\n'
        _, _ = bufio.writer_write(&out, line[:take+1])
        offset = (offset+take)%len(ALU)
        remaining -= take
    }
    seed: u32 = 42
    _, _ = bufio.writer_write_string(&out, ">TWO IUB ambiguity codes\n")
    random_fasta(&out, 3*n, "acgtBDHKMNRSVWY", []f64{0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02}, &seed)
    _, _ = bufio.writer_write_string(&out, ">THREE Homo sapiens frequency\n")
    random_fasta(&out, 5*n, "acgt", []f64{0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008}, &seed)
    assert(bufio.writer_flush(&out) == nil)
}
