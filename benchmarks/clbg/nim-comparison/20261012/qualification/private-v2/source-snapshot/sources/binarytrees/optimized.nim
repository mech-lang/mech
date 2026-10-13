import std/[os, strutils, typedthreads]

type Node {.acyclic.} = ref object
  left: Node
  right: Node
type Job = tuple[depth: int, first: int, last: int, output: ptr int]

proc make(depth: int): Node =
  new result
  if depth > 0:
    result.left = make(depth - 1)
    result.right = make(depth - 1)

proc check(node: Node): int =
  result = 1
  if node.left != nil:
    result += check(node.left) + check(node.right)

proc trees(job: Job) {.thread.} =
  var total = 0
  for iteration in job.first ..< job.last:
    let tree = make(job.depth)
    total += check(tree)
  job.output[] = total

let depth = max(6, parseInt(paramStr(1)))
block:
  let stretch = make(depth + 1)
  echo "stretch tree of depth ", depth + 1, "\t check: ", check(stretch)
let longLived = make(depth)
for current in countup(4, depth, 2):
  let iterations = 1 shl (depth - current + 4)
  let count = min(8, iterations)
  var totals: array[8, int]
  var threads: array[8, Thread[Job]]
  for worker in 0 ..< count:
    createThread(threads[worker], trees, (current, iterations * worker div count, iterations * (worker + 1) div count, addr totals[worker]))
  for worker in 0 ..< count:
    joinThread(threads[worker])
  var total = 0
  for worker in 0 ..< count:
    total += totals[worker]
  echo iterations, "\t trees of depth ", current, "\t check: ", total
echo "long lived tree of depth ", depth, "\t check: ", check(longLived)
