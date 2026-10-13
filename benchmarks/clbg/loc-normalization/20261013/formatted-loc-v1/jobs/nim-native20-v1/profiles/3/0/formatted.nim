import std/[os, strutils, typedthreads]

type Result = tuple[checksum: int, maximum: int]
type Job = tuple[n: int, first: int, last: int, output: ptr Result]

proc run(job: Job) {.thread.} =
  var factorial: array[13, int]
  factorial[0] = 1
  for width in 1 .. job.n:
    factorial[width] = factorial[width - 1] * width
  var permutation: array[12, int]
  var rotations: array[12, int]
  for i in 0 ..< job.n:
    permutation[i] = i
  for width in countdown(job.n, 2):
    let shift = (job.first div factorial[width - 1]) mod width
    rotations[width - 1] = shift
    for turn in 0 ..< shift:
      let first = permutation[0]
      for j in 0 ..< width - 1:
        permutation[j] = permutation[j + 1]
      permutation[width - 1] = first
  var checksum = 0
  var maximum = 0
  for rank in job.first ..< job.last:
    var work = permutation
    var flips = 0
    while work[0] != 0:
      var left = 0
      var right = work[0]
      while left < right:
        swap(work[left], work[right])
        inc left
        dec right
      inc flips
    checksum += (if (rank and 1) == 0: flips else: -flips)
    maximum = max(maximum, flips)
    for width in 2 .. job.n:
      let first = permutation[0]
      for j in 0 ..< width - 1:
        permutation[j] = permutation[j + 1]
      permutation[width - 1] = first
      inc rotations[width - 1]
      if rotations[width - 1] < width:
        break
      rotations[width - 1] = 0
  job.output[] = (checksum, maximum)

let n = parseInt(paramStr(1))
doAssert n >= 1 and n <= 12
var total = 1
for width in 1 .. n:
  total *= width
let count = min(8, total)
var threads: array[8, Thread[Job]]
var results: array[8, Result]
for worker in 0 ..< count:
  createThread(threads[worker], run, (n, total * worker div count, total * (
      worker + 1) div count, addr results[worker]))
for worker in 0 ..< count:
  joinThread(threads[worker])
var checksum = 0
var maximum = 0
for worker in 0 ..< count:
  checksum += results[worker].checksum
  maximum = max(maximum, results[worker].maximum)
echo checksum
echo "Pfannkuchen(", n, ") = ", maximum
