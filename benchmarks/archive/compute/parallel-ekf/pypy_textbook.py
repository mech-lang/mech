#!/usr/bin/env pypy3
from __future__ import annotations

import math as m
import sys
import time


D = 0.1
Q = [[0.01, 0.0], [0.0, 0.0025]]
R = 0.25
I = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
L = [140.0, 12.0]
E = 1.0e-4


def T(a):
    return [list(x) for x in zip(*a)]


def M(a, b):
    c = T(b)
    return [[sum(x * y for x, y in zip(r, q)) for q in c] for r in a]


def V(a, b):
    return [sum(x * y for x, y in zip(r, b)) for r in a]


def A(a, b):
    return [[x + y for x, y in zip(r, q)] for r, q in zip(a, b)]


def S(a, b):
    return [[x - y for x, y in zip(r, q)] for r, q in zip(a, b)]


def O(a, b):
    return [[x * y for y in b] for x in a]


def U(a, b):
    return [x + y for x, y in zip(a, b)]


def W(a, b):
    return [x - y for x, y in zip(a, b)]


def C(a, b):
    return [x * b for x in a]


def N(a, b):
    return [[x * b for x in r] for r in a]


def K(x, p):
    if not all(m.isfinite(v) for v in x):
        return False
    if not all(m.isfinite(v) for r in p for v in r):
        return False
    if p[0][0] <= 0.0 or p[1][1] <= 0.0 or p[2][2] <= 0.0:
        return False
    return all(abs(p[i][j] - p[j][i]) <= E for i, j in ((0, 1), (0, 2), (1, 2)))


def F(x, p, v, w, z, c):
    t = x[2]
    s = m.sin(t)
    q = m.cos(t)
    d = v * D
    y = U(x, [d * q, d * s, w * D])
    f = [[1.0, 0.0, -d * s], [0.0, 1.0, d * q], [0.0, 0.0, 1.0]]
    g = [[q * D, 0.0], [s * D, 0.0], [0.0, D]]
    pp = A(M(M(f, p), T(f)), M(M(g, Q), T(g)))
    e = W(L, y[:2])
    r = sum(a * a for a in e)
    h = [e[1] / r, -e[0] / r, -1.0]
    n = z - (m.atan2(e[1], e[0]) - y[2])
    n = m.atan2(m.sin(n), m.cos(n))
    j = V(pp, h)
    q = sum(a * b for a, b in zip(h, j)) + R
    k = C(j, 1.0 / q)
    y = U(y, C(k, n))
    a = S(I, O(k, h))
    cp = A(M(M(a, pp), T(a)), N(O(k, k), R))
    if c and not K(y, cp):
        return 1
    x[:] = y
    for i in range(3):
        p[i][:] = cp[i]
    return 0


def Z(x, p):
    for i in range(len(x)):
        x[i][:] = [55.0, 25.0, 0.4]
        p[i][:] = [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]]


def G(n, x, p, v, w, z, c):
    f = 0
    for _ in range(n):
        for i in range(len(x)):
            f += F(x[i], p[i], v[i], w[i], z[i], c)
    return f


def main():
    n = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10000)
    q = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    z = sys.argv[3].lower() if len(sys.argv) > 3 else "unchecked"
    if z not in {"checked", "unchecked"}:
        raise SystemExit("mode must be checked or unchecked")
    c = z == "checked"
    h = 2.0 * m.pi / n
    v = []
    w = []
    b = []
    for i in range(n):
        t = h * i
        v.append(1.0 + 0.05 * m.sin(t * 3.0))
        w.append(0.015 * (1.0 + 0.1 * m.sin(t * 2.0)))
        b.append(-0.55 + 0.01 * m.sin(t * 7.0) + 0.005 * m.sin(t * 11.0))
    x = [[55.0, 25.0, 0.4] for _ in range(n)]
    p = [[[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]] for _ in range(n)]
    G(5, x, p, v, w, b, c)
    Z(x, p)
    t = time.perf_counter()
    f = G(q, x, p, v, w, b, c)
    e = time.perf_counter() - t
    s = sum(sum(a) for a in x) + sum(sum(v for r in a for v in r) for a in p)
    print("lane: textbook-fidelity scalar outer loop")
    print(f"mode: {z}")
    print(f"instances: {n}")
    print(f"turns: {q}")
    print(f"elapsed_s: {e:.9f}")
    print(f"throughput: {n * q / e:.3f}")
    print(f"checksum: {s:.9f}")
    print(f"faults: {f}")


if __name__ == "__main__":
    main()
