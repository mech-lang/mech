from std.math import sqrt
from std.sys import argv
from max.algorithm.backend.cpu.parallelize import parallelize
from common import print_fixed

def coefficient[width: Int](i: SIMD[DType.int64, width], j: SIMD[DType.int64, width]) -> SIMD[DType.float64, width]:
    var ij = i + j
    return 1.0 / (ij * (ij + 1) // 2 + i + 1).cast[DType.float64]()

def multiply(imm u: List[Float64], mut out: List[Float64], transpose: Bool):
    var n = len(u)
    var src = u.unsafe_ptr()
    var dst = out.unsafe_ptr()
    def row(i: Int) {var}:
        var sum = SIMD[DType.float64, 2](0)
        var j = 0
        while j + 1 < n:
            var js = SIMD[DType.int64, 2](Int64(j), Int64(j + 1))
            var is_ = SIMD[DType.int64, 2](from_int=i)
            var a = coefficient[2](js, is_) if transpose else coefficient[2](is_, js)
            var values = SIMD[DType.float64, 2](src[j], src[j + 1])
            sum += values * a
            j += 2
        var s = sum.reduce_add()
        if j < n:
            s += src[j] * (coefficient[1](Int64(j), Int64(i)) if transpose else coefficient[1](Int64(i), Int64(j)))
        dst[i] = s
    parallelize(row, n, 8)

def av(imm u: List[Float64], mut out: List[Float64]):
    multiply(u, out, False)

def atv(imm u: List[Float64], mut out: List[Float64]):
    multiply(u, out, True)

def step(imm u: List[Float64], mut out: List[Float64], mut tmp: List[Float64]):
    av(u, tmp)
    atv(tmp, out)

def main() raises:
    var n = Int(argv()[1])
    var u = List[Float64]()
    var v = List[Float64]()
    var tmp = List[Float64]()
    u.resize(n, Float64(1))
    v.resize(n, Float64(0))
    tmp.resize(n, Float64(0))
    for _ in range(10):
        step(u, v, tmp)
        step(v, u, tmp)
    var uv: Float64 = 0
    var vv: Float64 = 0
    for i in range(n):
        uv += u[i] * v[i]
        vv += v[i] * v[i]
    print_fixed(sqrt(uv / vv), 9)
