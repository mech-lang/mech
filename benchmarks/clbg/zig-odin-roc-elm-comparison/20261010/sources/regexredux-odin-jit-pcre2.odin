package main
import "core:fmt"
import "core:os"
foreign import pcre2 "../support/libpcre2-8.0.dylib"
@(default_calling_convention="c", link_prefix="pcre2_")
foreign pcre2 {
    compile_8 :: proc(pattern: [^]u8, length: uint, options: u32, error_code: ^i32, error_offset: ^uint, ctx: rawptr) -> rawptr ---
    code_free_8 :: proc(code: rawptr) ---
    match_data_create_from_pattern_8 :: proc(code, ctx: rawptr) -> rawptr ---
    match_data_free_8 :: proc(data: rawptr) ---
    match_8 :: proc(code: rawptr, input: [^]u8, length, offset: uint, options: u32, data, ctx: rawptr) -> i32 ---
    get_ovector_pointer_8 :: proc(data: rawptr) -> [^]uint ---
    jit_compile_8 :: proc(code: rawptr, options: u32) -> i32 ---
}
Regex :: struct { code, data: rawptr }
compile :: proc(pattern: string, caseless: bool) -> Regex {
    error_code: i32
    error_offset: uint
    code := compile_8(raw_data(pattern), uint(len(pattern)), 8 if caseless else 0, &error_code, &error_offset, nil)
    assert(code != nil)
    status := jit_compile_8(code, 1)
    assert(status == 0)
    data := match_data_create_from_pattern_8(code, nil)
    assert(data != nil)
    return Regex{code, data}
}
destroy :: proc(regex: Regex) {
    match_data_free_8(regex.data)
    code_free_8(regex.code)
}
find :: proc(regex: Regex, input: string, offset: int) -> (begin, end: int, matched: bool) {
    status := match_8(regex.code, raw_data(input), uint(len(input)), uint(offset), 0, regex.data, nil)
    if status == -1 { return 0, 0, false }
    assert(status >= 0)
    range := get_ovector_pointer_8(regex.data)
    return int(range[0]), int(range[1]), true
}
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
    regex := compile(pattern, false)
    defer destroy(regex)
    output := make([dynamic]u8)
    offset := 0
    for {
        begin, end, matched := find(regex, input, offset)
        if !matched { break }
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
        regex := compile(pattern, true)
        count := 0
        offset := 0
        for {
            _, end, matched := find(regex, cleaned, offset)
            if !matched { break }
            count += 1
            offset = end
        }
        destroy(regex)
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
