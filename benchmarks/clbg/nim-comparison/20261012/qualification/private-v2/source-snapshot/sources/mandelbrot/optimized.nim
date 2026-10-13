import std/[os, strutils, typedthreads]

type Job = tuple[n: int, stride: int, first: int, last: int, image: ptr string]

proc rows(job: Job) {.thread.} =
  let scale = 2.0 / float64(job.n)
  for y in job.first ..< job.last:
    let ci = float64(y) * scale - 1.0
    for column in 0 ..< job.stride:
      let valid = min(8, job.n - column * 8)
      var cr: array[8, float64]
      var zr: array[8, float64]
      var zi: array[8, float64]
      var tr: array[8, float64]
      var ti: array[8, float64]
      for lane in 0 ..< valid:
        cr[lane] = float64(column * 8 + lane) * scale - 1.5
      for group in 0 ..< 10:
        for step in 0 ..< 5:
          for lane in 0 ..< valid:
            zi[lane] = (zr[lane] + zr[lane]) * zi[lane] + ci
            zr[lane] = tr[lane] - ti[lane] + cr[lane]
            tr[lane] = zr[lane] * zr[lane]
            ti[lane] = zi[lane] * zi[lane]
        var escaped = true
        for lane in 0 ..< valid:
          if tr[lane] + ti[lane] <= 4.0:
            escaped = false
        if escaped:
          break
      var value = 0
      for lane in 0 ..< valid:
        if tr[lane] + ti[lane] <= 4.0:
          value = value or (128 shr lane)
      job.image[][y * job.stride + column] = char(value)

let n = parseInt(paramStr(1))
doAssert n > 0
let stride = (n + 7) div 8
let count = min(8, n)
var image = newString(n * stride)
var threads: array[8, Thread[Job]]
for worker in 0 ..< count:
  createThread(threads[worker], rows, (n, stride, n * worker div count, n * (worker + 1) div count, addr image))
for worker in 0 ..< count:
  joinThread(threads[worker])
stdout.write("P4\n", n, " ", n, "\n")
stdout.write(image)
