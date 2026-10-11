from std.sys import argv
from common import write_string

@fieldwise_init
struct Node(ImplicitlyCopyable, Movable):
    var left: Int
    var right: Int

def build(depth: Int, mut nodes: List[Node]) -> Int:
    var left = build(depth - 1, nodes) if depth else 0
    var right = build(depth - 1, nodes) if depth else 0
    nodes.append(Node(left, right))
    return len(nodes)

def check(imm nodes: List[Node], root: Int) -> Int:
    return 1 + check(nodes, nodes[root - 1].left) + check(nodes, nodes[root - 1].right) if root else 0

def main() raises:
    var depth = max(6, Int(argv()[1]))
    var nodes = List[Node]()
    var root = build(depth + 1, nodes)
    write_string("stretch tree of depth " + String(depth + 1) + "\t check: " + String(check(nodes, root)) + "\n")
    nodes.clear()
    var long_lived = List[Node]()
    var long_root = build(depth, long_lived)
    for d in range(4, depth + 1, 2):
        var iterations = 1 << (depth - d + 4)
        var total = 0
        for _ in range(iterations):
            nodes.clear()
            root = build(d, nodes)
            total += check(nodes, root)
        write_string(String(iterations) + "\t trees of depth " + String(d) + "\t check: " + String(total) + "\n")
    write_string("long lived tree of depth " + String(depth) + "\t check: " + String(check(long_lived, long_root)) + "\n")
