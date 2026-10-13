import std/[os, strutils, math, typedthreads]

type Job = tuple[input: ptr seq[float64], output: ptr seq[float64], first: int,
    last: int, transpose: bool]

proc denominator(i, j: int): float64 {.inline.} =
  float64((i + j) * (i + j + 1) div 2 + i + 1)

proc rows(job: Job) {.thread.} =
  for i in job.first ..< job.last:
    var value = 0.0
    for j in 0 ..< job.input[].len:
      value += job.input[][j] / (if job.transpose: denominator(j,
          i) else: denominator(i, j))
    job.output[][i] = value

proc multiply(input: var seq[float64], output: var seq[float64],
    transpose: bool) =
  let count = min(8, input.len)
  var threads: array[8, Thread[Job]]
  for worker in 0 ..< count:
    createThread(threads[worker], rows, (addr input, addr output, input.len *
        worker div count, input.len * (worker + 1) div count, transpose))
  for worker in 0 ..< count:
    joinThread(threads[worker])

proc product(input: var seq[float64], temporary: var seq[float64],
    output: var seq[float64]) =
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
