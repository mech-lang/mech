from std.sys import argv
from max.algorithm.backend.cpu.parallelize import parallelize
from common import write_bytes, write_string
comptime Vec = SIMD[DType.float64, 4]

def main() raises:
    var n = Int(argv()[1])
    var stride = (n + 7) // 8
    var bytes = List[UInt8](unsafe_uninit_length=n * stride)
    var dst = bytes.unsafe_ptr()
    def row(y: Int) {var}:
        var ci = Vec(Float64(y) * 2.0 / Float64(n) - 1.0)
        for byte in range(stride):
            var bits: UInt8 = 0
            for group in range(2):
                var x = byte * 8 + group * 4
                var cr = Vec(Float64(x), Float64(x + 1), Float64(x + 2), Float64(x + 3)) * 2.0 / Float64(n) - 1.5
                var zr = Vec(0)
                var zi = Vec(0)
                var rr = Vec(0)
                var ii = Vec(0)
                var active = SIMD[DType.bool, 4](fill=True)
                for _ in range(50):
                    zi = 2.0 * zr * zi + ci
                    zr = rr - ii + cr
                    rr = zr * zr
                    ii = zi * zi
                    active &= (rr + ii).le(Vec(4.0))
                    if not active.reduce_or():
                        break
                comptime for b in range(4):
                    bits = (bits << 1) | UInt8(active[b] and x + b < n)
            dst[y * stride + byte] = bits
    parallelize(row, n, 8)
    write_string("P4\n" + String(n) + " " + String(n) + "\n")
    write_bytes(bytes)
