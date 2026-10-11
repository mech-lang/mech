package main
import "core:bufio"
import "core:os"
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
complement :: proc(byte: u8) -> u8 {
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
        header_end := record + strings.index(transmute(string)input[record:], "\n")
        _, _ = bufio.writer_write(&out, input[record:header_end+1])
        begin := header_end+1
        next := strings.index(transmute(string)input[begin:], ">")
        end := len(input) if next < 0 else begin+next
        used := 0
        for byte in input[begin:end] {
            if byte != '\n' && byte != '\r' {
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
                line[column] = '\n'
                _, _ = bufio.writer_write(&out, line[:column+1])
                column = 0
            }
        }
        record = end
    }
    assert(bufio.writer_flush(&out) == nil)
}
