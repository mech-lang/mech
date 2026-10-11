from std.sys import argv
from integer import integer, close, add, subtract, multiply, quotient, small, compare, multiply_add
from common import write_string

def extract(q: UInt64, r: UInt64, t: UInt64, scratch: UInt64, j: UInt64) -> UInt64:
    multiply_add(scratch, q, j, r)
    quotient(scratch, scratch, t)
    return small(scratch)

def main() raises:
    var n = Int(argv()[1])
    var q = integer(1)
    var r = integer(0)
    var t = integer(1)
    var scratch = integer(0)
    var k: UInt64 = 0
    var written = 0
    var text = String()
    while written < n:
        k += 1
        multiply_add(r, q, 2, r)
        multiply(r, r, 2 * k + 1)
        multiply(q, q, k)
        multiply(t, t, 2 * k + 1)
        if compare(q, r) > 0:
            continue
        var d = extract(q, r, t, scratch, 3)
        if d != extract(q, r, t, scratch, 4):
            continue
        text += chr(Int(d) + 48)
        written += 1
        if written % 10 == 0 or written == n:
            if written % 10 != 0:
                for _ in range(10 - written % 10):
                    text += " "
            write_string(text + "\t:" + String(written) + "\n")
            text = String()
        multiply(scratch, t, d)
        subtract(r, r, scratch)
        multiply(r, r, 10)
        multiply(q, q, 10)
    close(q)
    close(r)
    close(t)
    close(scratch)
