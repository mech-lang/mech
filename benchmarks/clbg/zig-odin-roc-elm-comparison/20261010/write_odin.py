"""Faithful Odin ports using only general standard libraries and GMP."""
import json
from bootstrap import D, sha

INPUT = '''read_all :: proc() -> [dynamic]u8 {
    bytes := make([dynamic]u8)
    buffer: [65536]u8
    for {
        n, err := os.read(os.stdin, buffer[:])
        if n == 0 { break }
        assert(err == nil)
        append(&bytes, ..buffer[:n])
    }
    return bytes
}
'''

FANNKUCH = '''package main
import "core:fmt"
import "core:os"
import "core:strconv"
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 7
    assert(n <= 16)
    perm: [16]int
    scratch: [16]int
    count: [16]int
    for i in 0..<n { perm[i] = i }
    r := n
    index := 0
    checksum := 0
    maximum := 0
    for {
        for r > 1 {
            count[r-1] = r
            r -= 1
        }
        copy(scratch[:n], perm[:n])
        flips := 0
        for scratch[0] != 0 {
            k := scratch[0]
            for i in 0..<(k+1)/2 {
                scratch[i], scratch[k-i] = scratch[k-i], scratch[i]
            }
            flips += 1
        }
        maximum = max(maximum, flips)
        if index%2 == 0 { checksum += flips } else { checksum -= flips }
        for {
            if r == n {
                fmt.printf("%d\\nPfannkuchen(%d) = %d\\n", checksum, n, maximum)
                return
            }
            first := perm[0]
            for i in 0..<r { perm[i] = perm[i+1] }
            perm[r] = first
            count[r] -= 1
            if count[r] > 0 { break }
            r += 1
        }
        index += 1
    }
}
'''

FASTA = '''package main
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
            SEARCH
            line[i] = letters[choice]
        }
        line[take] = '\\n'
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
    _, _ = bufio.writer_write_string(&out, ">ONE Homo sapiens alu\\n")
    alu := ALU
    line: [61]u8
    offset := 0
    remaining := 2*n
    for remaining > 0 {
        take := min(60, remaining)
        for i in 0..<take { line[i] = alu[(offset+i)%len(alu)] }
        line[take] = '\\n'
        _, _ = bufio.writer_write(&out, line[:take+1])
        offset = (offset+take)%len(ALU)
        remaining -= take
    }
    seed: u32 = 42
    _, _ = bufio.writer_write_string(&out, ">TWO IUB ambiguity codes\\n")
    random_fasta(&out, 3*n, "acgtBDHKMNRSVWY", []f64{0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02}, &seed)
    _, _ = bufio.writer_write_string(&out, ">THREE Homo sapiens frequency\\n")
    random_fasta(&out, 5*n, "acgt", []f64{0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008}, &seed)
    assert(bufio.writer_flush(&out) == nil)
}
'''

KNUCLEOTIDE = '''package main
import "core:fmt"
import "core:os"
import "core:slice"
import "core:strings"
''' + INPUT + '''encode_byte :: proc(byte: u8) -> u64 {
        switch byte {
        case 'C', 'c': return 1
        case 'G', 'g': return 2
        case 'T', 't': return 3
        }
        return 0
}
encode :: proc(text: string) -> u64 {
    key: u64
    for byte in transmute([]u8)text {
        key = key*4 + encode_byte(byte)
    }
    return key
}
decode :: proc(key: u64, width: int) -> string {
    bytes := make([]u8, width)
    alphabet := "ACGT"
    rest := key
    for i := width-1; i >= 0; i -= 1 {
        bytes[i] = alphabet[rest%4]
        rest /= 4
    }
    return transmute(string)bytes
}
Entry :: struct { key: u64, count: int }
less :: proc(a,b: Entry) -> bool {
    return a.count > b.count || (a.count == b.count && a.key < b.key)
}
main :: proc() {
    input := read_all()
    defer delete(input)
    start := strings.index(transmute(string)input[:], ">THREE")
    assert(start >= 0)
    header_end := start + strings.index(transmute(string)input[start:], "\\n")
    used := 0
    for byte in input[header_end+1:] {
        if byte == '>' { break }
        if byte == '\\n' || byte == '\\r' { continue }
        input[used] = u8(encode_byte(byte))
        used += 1
    }
    widths := [7]int{1,2,3,4,6,12,18}
    queries := [5]string{"GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"}
    for width, index in widths {
        counts := make(map[u64]int)
        mask := (u64(1) << u64(2*width))-1
        key: u64
        for byte, i in input[:used] {
            key = (key*4 + u64(byte)) & mask
            if i+1 >= width { counts[key] += 1 }
        }
        if width <= 2 {
            entries := make([dynamic]Entry)
            for key, count in counts { append(&entries, Entry{key,count}) }
            slice.sort_by(entries[:], less)
            for entry in entries {
                text := decode(entry.key, width)
                fmt.printf("%s %.3f\\n", text, f64(entry.count)*100.0/f64(used-width+1))
                delete(transmute([]u8)text)
            }
            fmt.println()
            delete(entries)
        } else {
            query := queries[index-2]
            fmt.printf("%d\\t%s\\n", counts[encode(query)], query)
        }
        delete(counts)
    }
}
'''

REVCOMP = '''package main
import "core:bufio"
import "core:os"
import "core:strings"
''' + INPUT + '''complement :: proc(byte: u8) -> u8 {
    switch byte {
    case 'a','A': return 'T'
    case 't','T','u','U': return 'A'
    case 'c','C': return 'G'
    case 'g','G': return 'C'
    case 'm','M': return 'K'
    case 'r','R': return 'Y'
    case 'w','W': return 'W'
    case 's','S': return 'S'
    case 'y','Y': return 'R'
    case 'k','K': return 'M'
    case 'v','V': return 'B'
    case 'h','H': return 'D'
    case 'd','D': return 'H'
    case 'b','B': return 'V'
    case 'n','N': return 'N'
    }
    panic("invalid DNA")
}
main :: proc() {
    input := read_all()
    defer delete(input)
    out: bufio.Writer
    bufio.writer_init(&out, os.to_stream(os.stdout), 65536)
    defer bufio.writer_destroy(&out)
    record := 0
    for record < len(input) {
        header_end := record + strings.index(transmute(string)input[record:], "\\n")
        _, _ = bufio.writer_write(&out, input[record:header_end+1])
        begin := header_end+1
        next := strings.index(transmute(string)input[begin:], ">")
        end := len(input) if next < 0 else begin+next
        used := 0
        for byte in input[begin:end] {
            if byte != '\\n' && byte != '\\r' {
                input[begin+used] = byte
                used += 1
            }
        }
        line: [61]u8
        column := 0
        for used > 0 {
            used -= 1
            line[column] = complement(input[begin+used])
            column += 1
            if column == 60 || used == 0 {
                line[column] = '\\n'
                _, _ = bufio.writer_write(&out, line[:column+1])
                column = 0
            }
        }
        record = end
    }
    assert(bufio.writer_flush(&out) == nil)
}
'''

REGEX = '''package main
import "core:fmt"
import "core:os"
import "core:text/regex"
''' + INPUT + '''substitute :: proc(input, pattern, replacement: string) -> string {
    iterator, err := regex.create_iterator(input, pattern)
    assert(err == nil)
    defer regex.destroy(iterator)
    output := make([dynamic]u8)
    offset := 0
    for capture, _ in regex.match(&iterator) {
        begin, end := capture.pos[0][0], capture.pos[0][1]
        append(&output, ..transmute([]u8)input[offset:begin])
        append(&output, ..transmute([]u8)replacement)
        offset = end
    }
    append(&output, ..transmute([]u8)input[offset:])
    return transmute(string)output[:]
}
main :: proc() {
    input := read_all()
    defer delete(input)
    raw := transmute(string)input[:]
    cleaned := substitute(raw, ">[^\\n]*\\n|\\n", "")
    patterns := [9]string{
        "agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct",
    }
    for pattern in patterns {
        iterator, err := regex.create_iterator(cleaned, pattern, {.Case_Insensitive})
        assert(err == nil)
        count := 0
        for _, _ in regex.match(&iterator) { count += 1 }
        regex.destroy(iterator)
        fmt.printf("%s %d\\n", pattern, count)
    }
    replacements := [5][2]string{
        {"tHa[Nt]", "<4>"}, {"aND|caN|Ha[DS]|WaS", "<3>"}, {"a[NSt]|BY", "<2>"},
        {"<[^>]*>", "|"}, {"\\\\|[^|][^|]*\\\\|", "-"},
    }
    replaced := cleaned
    for replacement in replacements {
        next := substitute(replaced, replacement[0], replacement[1])
        if raw_data(replaced) != raw_data(cleaned) { delete(transmute([]u8)replaced) }
        replaced = next
    }
    fmt.printf("\\n%d\\n%d\\n%d\\n", len(raw), len(cleaned), len(replaced))
    delete(transmute([]u8)replaced)
    delete(transmute([]u8)cleaned)
}
'''

GMP = '''package main
import "core:fmt"
import "core:os"
import "core:strconv"
foreign import gmp "external:libgmp.10.dylib"
Integer :: struct { allocated, size: i32, limbs: rawptr }
@(default_calling_convention="c", link_prefix="__gmpz_")
foreign gmp {
    init :: proc(x: ^Integer) ---
    clear :: proc(x: ^Integer) ---
    set_ui :: proc(x: ^Integer, value: u64) ---
    mul_ui :: proc(x, a: ^Integer, value: u64) ---
    add :: proc(x,a,b: ^Integer) ---
    sub :: proc(x,a,b: ^Integer) ---
    fdiv_q :: proc(x,a,b: ^Integer) ---
    cmp :: proc(a,b: ^Integer) -> i32 ---
    get_ui :: proc(a: ^Integer) -> u64 ---
}
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 30
    values: [7]Integer
    for &value in values { init(&value) }
    defer for &value in values { clear(&value) }
    q := &values[0]
    r := &values[1]
    t := &values[2]
    a := &values[3]
    b := &values[4]
    first := &values[5]
    second := &values[6]
    set_ui(q,1)
    set_ui(t,1)
    k := 0
    produced := 0
    line: [10]u8
    for produced < n {
        mul_ui(a,q,3)
        add(a,a,r)
        fdiv_q(first,a,t)
        mul_ui(b,q,4)
        add(b,b,r)
        fdiv_q(second,b,t)
        if cmp(first,second) == 0 {
            digit := get_ui(first)
            line[produced%10] = u8(digit)+'0'
            produced += 1
            if produced%10 == 0 || produced == n {
                used := 10 if produced%10 == 0 else produced%10
                for i in used..<10 { line[i] = ' ' }
                fmt.printf("%s\\t:%d\\n", transmute(string)line[:], produced)
            }
            mul_ui(a,t,digit)
            sub(r,r,a)
            mul_ui(r,r,10)
            mul_ui(q,q,10)
        } else {
            k += 1
            multiplier := u64(2*k+1)
            mul_ui(a,q,2)
            add(r,r,a)
            mul_ui(r,r,multiplier)
            mul_ui(q,q,u64(k))
            mul_ui(t,t,multiplier)
        }
    }
}
'''

def main():
    rows = []
    def emit(benchmark, variant, source, libraries=None):
        identifier = benchmark + '-odin-' + variant
        path = D / 'sources' / (identifier + '.odin')
        path.write_text(source)
        rows.append(dict(key=benchmark + ':' + identifier, id=identifier, benchmark=benchmark, language='odin',
            source=str(path), source_sha256=sha(path), local_adaptation=True, adaptations=['Locally authored faithful workload using only general libraries; application sequencing, helpers, and FFI bindings count toward LOC.'],
            source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/' + benchmark + '.html', libraries=libraries or []))
    emit('fannkuchredux','compact',FANNKUCH)
    emit('fasta','linear',FASTA.replace('SEARCH','for cumulative[choice] <= random { choice += 1 }'))
    emit('fasta','binary',FASTA.replace('SEARCH','''low := 0
            high := len(probabilities)-1
            for low < high {
                middle := (low+high)/2
                if random < cumulative[middle] { high = middle } else { low = middle+1 }
            }
            choice = low'''))
    emit('knucleotide','growable-maps',KNUCLEOTIDE)
    emit('revcomp','compact',REVCOMP)
    emit('regexredux','stdlib',REGEX)
    emit('pidigits','gmp',GMP,['gmp'])
    (D / 'written-odin.json').write_text(json.dumps(dict(programs=rows),indent=2)+'\n')
    print('WROTE ODIN',len(rows),flush=True)

if __name__ == '__main__': main()
