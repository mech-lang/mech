import std/[os, strutils]

type Node = ref object
  left: Node
  right: Node

proc make(depth: int): Node =
  new result
  if depth > 0:
    result.left = make(depth - 1)
    result.right = make(depth - 1)

proc check(node: Node): int =
  result = 1
  if node.left != nil:
    result += check(node.left) + check(node.right)

let depth = max(6, parseInt(paramStr(1)))
block:
  let stretch = make(depth + 1)
  echo "stretch tree of depth ", depth + 1, "\t check: ", check(stretch)
let longLived = make(depth)
for current in countup(4, depth, 2):
  let iterations = 1 shl (depth - current + 4)
  var total = 0
  for iteration in 0 ..< iterations:
    let tree = make(current)
    total += check(tree)
  echo iterations, "\t trees of depth ", current, "\t check: ", total
echo "long lived tree of depth ", depth, "\t check: ", check(longLived)
