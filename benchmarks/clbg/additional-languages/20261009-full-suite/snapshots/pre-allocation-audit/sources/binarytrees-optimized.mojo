from std.sys import argv
from std.memory import UnsafePointer
from max.algorithm.backend.cpu.parallelize import parallelize
from common import write_string, opaque_int

@fieldwise_init
struct Node(ImplicitlyCopyable, Movable):
    var left: Int32
    var right: Int32

@no_inline
def build[origin: MutOrigin](depth: Int, var nodes: UnsafePointer[Node, origin], root: Int):
    var size = (1 << (depth + 1)) - 1
    for i in range(size):
        var left = 2 * i + 1 if 2 * i + 1 < size else -1
        var right = 2 * i + 2 if 2 * i + 2 < size else -1
        nodes[root + i] = Node(Int32(left), Int32(right))

@no_inline
def check[origin: Origin](nodes: UnsafePointer[Node, origin], root: Int) -> Int:
    if root < 0:
        return 0
    var node = nodes[root]
    if node.left < 0 and node.right < 0:
        return 1
    return 1 + check(nodes, Int(node.left)) + check(nodes, Int(node.right))

def main() raises:
    var depth = max(6, Int(argv()[1]))
    assert depth < 30, "Tree storage must fit signed 32-bit child indices"
    var stretch = List[Node](unsafe_uninit_length=(1 << (depth + 2)) - 1)
    build(depth + 1, stretch.unsafe_ptr(), 0)
    write_string("stretch tree of depth " + String(depth + 1) + "\t check: " + String(check(stretch.unsafe_ptr(), 0)) + "\n")
    stretch.clear()
    var long_lived = List[Node](unsafe_uninit_length=(1 << (depth + 1)) - 1)
    build(depth, long_lived.unsafe_ptr(), 0)
    for d in range(4, depth + 1, 2):
        var iterations = 1 << (depth - d + 4)
        var sums = List[Int](length=8, fill=0)
        var dst = sums.unsafe_ptr()
        def worker(w: Int) {var}:
            var nodes = List[Node](unsafe_uninit_length=(1 << (d + 1)) - 1)
            var sum = 0
            for _ in range(w * iterations // 8, (w + 1) * iterations // 8):
                build(opaque_int(d), nodes.unsafe_ptr(), 0)
                sum += check(nodes.unsafe_ptr(), 0)
            dst[w] = sum
        parallelize(worker, 8, 8)
        var sum = 0
        for s in sums:
            sum += s
        write_string(String(iterations) + "\t trees of depth " + String(d) + "\t check: " + String(sum) + "\n")
    write_string("long lived tree of depth " + String(depth) + "\t check: " + String(check(long_lived.unsafe_ptr(), 0)) + "\n")
