from std.sys import argv
from std.memory import Pointer
from std.origin import MutUntrackedOrigin
from max.algorithm.backend.cpu.parallelize import parallelize
from common import write_string
from pool import Pool, initialize, terminate

@fieldwise_init
struct Node(ImplicitlyCopyable, Movable):
    var left: UInt64
    var right: UInt64

def build(depth: Int, pool: Pool) -> UInt64:
    var address = pool.allocate_bytes(16)
    var node = Pointer[Node, MutUntrackedOrigin](unsafe_from_address=Int(address))
    var left = build(depth - 1, pool) if depth else UInt64(0)
    var right = build(depth - 1, pool) if depth else UInt64(0)
    node.unsafe_write(Node(left, right))
    return address

def check(address: UInt64) -> Int:
    var node = Pointer[Node, MutUntrackedOrigin](unsafe_from_address=Int(address))[]
    return 1 if node.left == 0 else 1 + check(node.left) + check(node.right)

def main() raises:
    initialize()
    var depth = max(6, Int(argv()[1]))
    var stretch = Pool()
    var root = build(depth + 1, stretch)
    write_string("stretch tree of depth " + String(depth + 1) + "\t check: " + String(check(root)) + "\n")
    stretch.destroy()
    var long_lived = Pool()
    var long_root = build(depth, long_lived)
    for d in range(4, depth + 1, 2):
        var iterations = 1 << (depth - d + 4)
        var sums = List[Int](length=8, fill=0)
        var dst = sums.unsafe_ptr()
        def worker(w: Int) {var}:
            var pool = Pool()
            var total = 0
            for _ in range(w * iterations // 8, (w + 1) * iterations // 8):
                total += check(build(d, pool))
                pool.clear()
            pool.destroy()
            dst[w] = total
        parallelize(worker, 8, 8)
        var total = 0
        for s in sums:
            total += s
        write_string(String(iterations) + "\t trees of depth " + String(d) + "\t check: " + String(total) + "\n")
    write_string("long lived tree of depth " + String(depth) + "\t check: " + String(check(long_root)) + "\n")
    long_lived.destroy()
    terminate()
