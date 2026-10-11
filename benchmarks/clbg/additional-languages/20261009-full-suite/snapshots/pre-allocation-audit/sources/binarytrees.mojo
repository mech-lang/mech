from std.sys import argv
from common import write_string
from max.algorithm.backend.cpu.parallelize import parallelize

@fieldwise_init
struct Node(ImplicitlyCopyable, Movable):
    var left: Int
    var right: Int

def build(depth: Int, mut nodes: List[Node]) -> Int:
    var left = 0
    var right = 0
    if depth > 0:
        left = build(depth - 1, nodes)
        right = build(depth - 1, nodes)
    nodes.append(Node(left, right))
    return len(nodes)

def check(imm nodes: List[Node], root: Int) -> Int:
    if root == 0:
        return 0
    var node = nodes[root - 1]
    return 1 + check(nodes, node.left) + check(nodes, node.right)

def main() raises:
    var depth = max(6, Int(argv()[1]))
    var stretch = List[Node]()
    var root = build(depth + 1, stretch)
    write_string("stretch tree of depth " + String(depth + 1) + "\t check: " + String(check(stretch, root)) + "\n")
    stretch.clear()
    var long_lived = List[Node]()
    var long_root = build(depth, long_lived)
    for d in range(4, depth + 1, 2):
        var iterations = 1 << (depth - d + 4)
        var sums: List[Int] = [0, 0, 0, 0, 0, 0, 0, 0]
        var dst = sums.unsafe_ptr()
        def worker(w: Int) {var}:
            var sum = 0
            for _ in range(w * iterations // 8, (w + 1) * iterations // 8):
                var nodes = List[Node](capacity=(1 << (d + 1)) - 1)
                var root = build(d, nodes)
                sum += check(nodes, root)
            dst[w] = sum
        parallelize(worker, 8, 8)
        var sum = 0
        for s in sums:
            sum += s
        write_string(String(iterations) + "\t trees of depth " + String(d) + "\t check: " + String(sum) + "\n")
    write_string("long lived tree of depth " + String(depth) + "\t check: " + String(check(long_lived, long_root)) + "\n")
