"""Spectral-norm — Mojo implementation.

Mojo is a compiled language with Python-like syntax. This is a direct
translation of the official Benchmarks Game spectral-norm algorithm.

Attribution:
  Based on the Benchmarks Game Python #6 implementation
  (https://benchmarksgame-team.pages.debian.net/benchmarksgame/)
"""

from std.math import sqrt


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

comptime DEFAULT_N = 2000


# ---------------------------------------------------------------------------
# Algorithm
# ---------------------------------------------------------------------------

fn sn_eval_A(i: Int, j: Int) -> Float64:
    var ij = i + j
    return 1.0 / Float64(ij * (ij + 1) // 2 + i + 1)


fn sn_eval_AtA_times_u(
    n: Int,
    u: List[Float64],
    mut out: List[Float64],
    mut tmp: List[Float64],
):
    # tmp = A * u
    for i in range(n):
        var s: Float64 = 0.0
        for j in range(n):
            s += sn_eval_A(i, j) * u[j]
        tmp[i] = s
    # out = A^T * tmp
    for i in range(n):
        var s: Float64 = 0.0
        for j in range(n):
            s += sn_eval_A(j, i) * tmp[j]
        out[i] = s


fn run_spectral(n: Int = DEFAULT_N) -> Float64:
    """Run spectral-norm computation. Returns the spectral norm value."""
    var u = List[Float64](capacity=n)
    var v = List[Float64](capacity=n)
    var tmp = List[Float64](capacity=n)

    for _ in range(n):
        u.append(1.0)
        v.append(0.0)
        tmp.append(0.0)

    for _ in range(10):
        sn_eval_AtA_times_u(n, u, v, tmp)
        sn_eval_AtA_times_u(n, v, u, tmp)

    var vBv: Float64 = 0.0
    var vv: Float64 = 0.0
    for i in range(n):
        vBv += u[i] * v[i]
        vv += v[i] * v[i]

    return sqrt(vBv / vv)
