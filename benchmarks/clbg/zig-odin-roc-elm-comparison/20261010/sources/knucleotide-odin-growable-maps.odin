package main
import "core:fmt"
import "core:os"
import "core:slice"
import "core:strings"
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
encode_byte :: proc(byte: u8) -> u64 {
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
    header_end := start + strings.index(transmute(string)input[start:], "\n")
    used := 0
    for byte in input[header_end+1:] {
        if byte == '>' { break }
        if byte == '\n' || byte == '\r' { continue }
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
                fmt.printf("%s %.3f\n", text, f64(entry.count)*100.0/f64(used-width+1))
                delete(transmute([]u8)text)
            }
            fmt.println()
            delete(entries)
        } else {
            query := queries[index-2]
            fmt.printf("%d\t%s\n", counts[encode(query)], query)
        }
        delete(counts)
    }
}
