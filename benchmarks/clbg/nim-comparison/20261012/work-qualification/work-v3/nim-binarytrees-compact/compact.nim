import std/sets
import audit_support
import std/[os, strutils]

type Node = ref object
  left: Node
  right: Node

proc make(depth: int): Node =
  new result
  if depth > 0:
    result.left = make(depth - 1)
    result.right = make(depth - 1)

proc auditTree(audit: File, node: Node, depth: int, kind: string, index: int) =
  audit.writeLine("ROOT|", kind, "|", depth, "|", index, "|", cast[uint](node))
  var seen = initHashSet[uint]()
  proc visit(current: Node) =
    doAssert current != nil
    let address = cast[uint](current)
    doAssert address notin seen
    seen.incl(address)
    audit.writeLine("NODE|", address, "|", cast[uint](current.left), "|", cast[uint](current.right))
    if current.left != nil:
      doAssert current.right != nil
      visit(current.left)
      visit(current.right)
    else:
      doAssert current.right == nil
  visit(node)
  audit.writeLine("TREE_END|", seen.len)

proc check(node: Node): int =
  result = 1
  if node.left != nil:
    result += check(node.left) + check(node.right)

let audit = auditOpen("main")
let depth = max(6, parseInt(paramStr(1)))
block:
  let stretch = make(depth + 1)
  auditTree(audit, stretch, depth + 1, "stretch", -1)
  echo "stretch tree of depth ", depth + 1, "\t check: ", check(stretch)
let longLived = make(depth)
auditTree(audit, longLived, depth, "long-open", -1)
for current in countup(4, depth, 2):
  let iterations = 1 shl (depth - current + 4)
  var total = 0
  for iteration in 0 ..< iterations:
    let tree = make(current)
    auditTree(audit, tree, current, "iteration", iteration)
    total += check(tree)
  echo iterations, "\t trees of depth ", current, "\t check: ", total
auditTree(audit, longLived, depth, "long-close", -1)
echo "long lived tree of depth ", depth, "\t check: ", check(longLived)

audit.writeLine("END")
audit.close()
