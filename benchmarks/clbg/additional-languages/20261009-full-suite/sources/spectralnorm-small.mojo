# Scalar port of cemrehancavdar/faster-python-bench spectral_norm.mojo.
# The prescribed four functions are separate; coefficients are not cached.
from std.math import sqrt
from std.sys import argv
from common import print_fixed

def coefficient(i: Int, j: Int) -> Float64:
    var ij = i + j
    return 1.0 / Float64(ij * (ij + 1) // 2 + i + 1)

def av(imm u: List[Float64], mut out: List[Float64]):
    for i in range(len(u)):
        var s: Float64 = 0
        for j in range(len(u)):
            s += coefficient(i, j) * u[j]
        out[i] = s

def atv(imm u: List[Float64], mut out: List[Float64]):
    for i in range(len(u)):
        var s: Float64 = 0
        for j in range(len(u)):
            s += coefficient(j, i) * u[j]
        out[i] = s

def step(imm u: List[Float64], mut out: List[Float64], mut tmp: List[Float64]):
    av(u, tmp)
    atv(tmp, out)

def main() raises:
    var n = Int(argv()[1])
    var u = List[Float64](length=n, fill=Float64(1))
    var v = List[Float64](length=n, fill=Float64(0))
    var tmp = List[Float64](length=n, fill=Float64(0))
    for _ in range(10):
        step(u, v, tmp)
        step(v, u, tmp)
    var uv: Float64 = 0
    var vv: Float64 = 0
    for i in range(n):
        uv += u[i] * v[i]
        vv += v[i] * v[i]
    print_fixed(sqrt(uv / vv), 9)
