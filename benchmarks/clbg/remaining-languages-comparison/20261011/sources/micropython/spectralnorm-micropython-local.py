# Inline complete implicit-matrix products; no precomputed workload kernel.
import sys
from math import sqrt
def multiply(x, transpose):
    n = len(x)
    out = [0.0]*n
    for i in range(n):
        total = 0.0
        for j in range(n):
            s = i+j
            denominator = s*(s+1)//2 + (j if transpose else i) + 1
            total += x[j] / denominator
        out[i] = total
    return out
def step(x):
    return multiply(multiply(x, False), True)
n = int(sys.argv[1])
u = [1.0]*n
v = [0.0]*n
for iteration in range(10):
    v = step(u)
    u = step(v)
print("%.9f" % sqrt(sum(a*b for a,b in zip(u,v))/sum(b*b for b in v)))
