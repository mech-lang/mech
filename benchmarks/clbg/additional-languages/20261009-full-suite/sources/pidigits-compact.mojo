from std.sys import argv
from integer import BigInt
from common import write_string

def main() raises:
    var n = Int(argv()[1])
    var q = BigInt(1)
    var r = BigInt(0)
    var t = BigInt(1)
    var k = 0
    var written = 0
    var text = String()
    while written < n:
        k += 1
        r = (r + q * 2) * (2 * k + 1)
        q = q * k
        t = t * (2 * k + 1)
        if q > r:
            continue
        var d = ((q * 3 + r) // t).to_int()
        if d != ((q * 4 + r) // t).to_int():
            continue
        text += chr(d + 48)
        written += 1
        if written % 10 == 0 or written == n:
            for _ in range((10 - written % 10) % 10):
                text += " "
            write_string(text + "\t:" + String(written) + "\n")
            text = String()
        r = (r - t * d) * 10
        q = q * 10
