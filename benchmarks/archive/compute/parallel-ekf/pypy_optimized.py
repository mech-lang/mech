#!/usr/bin/env pypy3
from __future__ import annotations

from array import array
import math as m
import sys
import time


D = 0.1
A = 0.01
B = 0.0025
R = 0.25
E = 1.0e-4


def Z(x, p):
    for i in range(len(x[0])):
        x[0][i] = 55.0
        x[1][i] = 25.0
        x[2][i] = 0.4
        for a in p:
            a[i] = 0.0
        p[0][i] = 100.0
        p[4][i] = 100.0
        p[8][i] = 0.15


def F(i, x, p, v, w, b, c):
    x0, x1, x2 = x
    p00, p01, p02, p10, p11, p12, p20, p21, p22 = p
    t = x2[i]
    s = m.sin(t)
    q = m.cos(t)
    d = v[i] * D
    y0 = x0[i] + d * q
    y1 = x1[i] + d * s
    y2 = t + w[i] * D
    fs = -d * s
    fc = d * q
    u00 = p00[i] + fs * p20[i]
    u01 = p01[i] + fs * p21[i]
    u02 = p02[i] + fs * p22[i]
    u10 = p10[i] + fc * p20[i]
    u11 = p11[i] + fc * p21[i]
    u12 = p12[i] + fc * p22[i]
    d2 = D * D
    a00 = u00 + u02 * fs + q * q * d2 * A
    a01 = u01 + u02 * fc + q * s * d2 * A
    a02 = u02
    a10 = u10 + u12 * fs + s * q * d2 * A
    a11 = u11 + u12 * fc + s * s * d2 * A
    a12 = u12
    a20 = p20[i] + p22[i] * fs
    a21 = p21[i] + p22[i] * fc
    a22 = p22[i] + d2 * B
    dx = 140.0 - y0
    dy = 12.0 - y1
    rr = dx * dx + dy * dy
    h0 = dy / rr
    h1 = -dx / rr
    n = b[i] - (m.atan2(dy, dx) - y2)
    n = m.atan2(m.sin(n), m.cos(n))
    j0 = a00 * h0 + a01 * h1 - a02
    j1 = a10 * h0 + a11 * h1 - a12
    j2 = a20 * h0 + a21 * h1 - a22
    q = h0 * j0 + h1 * j1 - j2 + R
    k0 = j0 / q
    k1 = j1 / q
    k2 = j2 / q
    y0 += k0 * n
    y1 += k1 * n
    y2 += k2 * n
    c00 = 1.0 - k0 * h0
    c01 = -k0 * h1
    c02 = k0
    c10 = -k1 * h0
    c11 = 1.0 - k1 * h1
    c12 = k1
    c20 = -k2 * h0
    c21 = -k2 * h1
    c22 = 1.0 + k2
    l00 = c00 * a00 + c01 * a10 + c02 * a20
    l01 = c00 * a01 + c01 * a11 + c02 * a21
    l02 = c00 * a02 + c01 * a12 + c02 * a22
    l10 = c10 * a00 + c11 * a10 + c12 * a20
    l11 = c10 * a01 + c11 * a11 + c12 * a21
    l12 = c10 * a02 + c11 * a12 + c12 * a22
    l20 = c20 * a00 + c21 * a10 + c22 * a20
    l21 = c20 * a01 + c21 * a11 + c22 * a21
    l22 = c20 * a02 + c21 * a12 + c22 * a22
    n00 = l00 * c00 + l01 * c01 + l02 * c02 + k0 * k0 * R
    n01 = l00 * c10 + l01 * c11 + l02 * c12 + k0 * k1 * R
    n02 = l00 * c20 + l01 * c21 + l02 * c22 + k0 * k2 * R
    n10 = l10 * c00 + l11 * c01 + l12 * c02 + k1 * k0 * R
    n11 = l10 * c10 + l11 * c11 + l12 * c12 + k1 * k1 * R
    n12 = l10 * c20 + l11 * c21 + l12 * c22 + k1 * k2 * R
    n20 = l20 * c00 + l21 * c01 + l22 * c02 + k2 * k0 * R
    n21 = l20 * c10 + l21 * c11 + l22 * c12 + k2 * k1 * R
    n22 = l20 * c20 + l21 * c21 + l22 * c22 + k2 * k2 * R
    if c and not (
        m.isfinite(y0) and m.isfinite(y1) and m.isfinite(y2)
        and m.isfinite(n00) and m.isfinite(n01) and m.isfinite(n02)
        and m.isfinite(n10) and m.isfinite(n11) and m.isfinite(n12)
        and m.isfinite(n20) and m.isfinite(n21) and m.isfinite(n22)
        and n00 > 0.0 and n11 > 0.0 and n22 > 0.0
        and abs(n01 - n10) <= E and abs(n02 - n20) <= E and abs(n12 - n21) <= E
    ):
        return 1
    x0[i] = y0
    x1[i] = y1
    x2[i] = y2
    p00[i] = n00
    p01[i] = n01
    p02[i] = n02
    p10[i] = n10
    p11[i] = n11
    p12[i] = n12
    p20[i] = n20
    p21[i] = n21
    p22[i] = n22
    return 0


def G(n, x, p, v, w, b, c):
    f = 0
    for _ in range(n):
        for i in range(len(v)):
            f += F(i, x, p, v, w, b, c)
    return f


def main():
    n = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10000)
    q = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    z = sys.argv[3].lower() if len(sys.argv) > 3 else "unchecked"
    if z not in {"checked", "unchecked"}:
        raise SystemExit("mode must be checked or unchecked")
    c = z == "checked"
    v = array("f", [0.0]) * n
    w = array("f", [0.0]) * n
    b = array("f", [0.0]) * n
    h = 2.0 * m.pi / n
    for i in range(n):
        t = h * i
        v[i] = 1.0 + 0.05 * m.sin(t * 3.0)
        w[i] = 0.015 * (1.0 + 0.1 * m.sin(t * 2.0))
        b[i] = -0.55 + 0.01 * m.sin(t * 7.0) + 0.005 * m.sin(t * 11.0)
    x = tuple(array("f", [0.0]) * n for _ in range(3))
    p = tuple(array("f", [0.0]) * n for _ in range(9))
    Z(x, p)
    G(5, x, p, v, w, b, c)
    Z(x, p)
    t = time.perf_counter()
    f = G(q, x, p, v, w, b, c)
    e = time.perf_counter() - t
    s = sum(map(float, x[0])) + sum(map(float, x[1])) + sum(map(float, x[2]))
    s += sum(sum(map(float, a)) for a in p)
    print("lane: optimized pure-Python scalar outer loop")
    print(f"mode: {z}")
    print(f"instances: {n}")
    print(f"turns: {q}")
    print(f"elapsed_s: {e:.9f}")
    print(f"throughput: {n * q / e:.3f}")
    print(f"checksum: {s:.9f}")
    print(f"faults: {f}")


if __name__ == "__main__":
    main()
