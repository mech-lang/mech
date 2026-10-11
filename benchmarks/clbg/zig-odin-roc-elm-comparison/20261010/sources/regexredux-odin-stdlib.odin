package main
import "core:fmt"
import "core:os"
import "core:text/regex"
read_all :: proc() -> [dynamic]u8 {
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
substitute :: proc(input, pattern, replacement: string) -> string {
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
    cleaned := substitute(raw, ">[^\n]*\n|\n", "")
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
        fmt.printf("%s %d\n", pattern, count)
    }
    replacements := [5][2]string{
        {"tHa[Nt]", "<4>"}, {"aND|caN|Ha[DS]|WaS", "<3>"}, {"a[NSt]|BY", "<2>"},
        {"<[^>]*>", "|"}, {"\\|[^|][^|]*\\|", "-"},
    }
    replaced := cleaned
    for replacement in replacements {
        next := substitute(replaced, replacement[0], replacement[1])
        if raw_data(replaced) != raw_data(cleaned) { delete(transmute([]u8)replaced) }
        replaced = next
    }
    fmt.printf("\n%d\n%d\n%d\n", len(raw), len(cleaned), len(replaced))
    delete(transmute([]u8)replaced)
    delete(transmute([]u8)cleaned)
}
