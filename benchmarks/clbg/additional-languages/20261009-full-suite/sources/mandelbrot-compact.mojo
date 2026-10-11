from std.sys import argv
from common import write_bytes, write_string

def main() raises:
    var n = Int(argv()[1])
    var out = List[UInt8]()
    for y in range(n):
        var ci = 2.0 * Float64(y) / Float64(n) - 1.0
        for byte in range((n + 7) // 8):
            var bits: UInt8 = 0
            for bit in range(8):
                var x = byte * 8 + bit
                var cr = 2.0 * Float64(x) / Float64(n) - 1.5
                var zr: Float64 = 0
                var zi: Float64 = 0
                var rr: Float64 = 0
                var ii: Float64 = 0
                for _ in range(50):
                    zi = 2.0 * zr * zi + ci
                    zr = rr - ii + cr
                    rr = zr * zr
                    ii = zi * zi
                    if rr + ii > 4.0:
                        break
                bits = (bits << 1) | UInt8(x < n and rr + ii <= 4.0)
            out.append(bits)
    write_string("P4\n" + String(n) + " " + String(n) + "\n")
    write_bytes(out)
