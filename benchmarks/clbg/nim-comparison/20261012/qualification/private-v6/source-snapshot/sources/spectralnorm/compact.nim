import std/[os, strutils, math]

proc denominator(i, j: int): float64 {.inline.} =
  float64((i + j) * (i + j + 1) div 2 + i + 1)

proc multiply(input: seq[float64], output: var seq[float64], transpose: bool) =
  for i in 0 ..< input.len:
    var value = 0.0
    for j in 0 ..< input.len:
      value += input[j] / (if transpose: denominator(j, i) else: denominator(i, j))
    output[i] = value

proc product(input: seq[float64], temporary: var seq[float64], output: var seq[float64]) =
  multiply(input, temporary, false)
  multiply(temporary, output, true)

let n = parseInt(paramStr(1))
doAssert n > 0
var u = newSeq[float64](n)
var v = newSeq[float64](n)
var temporary = newSeq[float64](n)
for i in 0 ..< n:
  u[i] = 1.0
for iteration in 0 ..< 10:
  product(u, temporary, v)
  product(v, temporary, u)
var uv = 0.0
var vv = 0.0
for i in 0 ..< n:
  uv += u[i] * v[i]
  vv += v[i] * v[i]
echo formatFloat(sqrt(uv / vv), ffDecimal, 9)
