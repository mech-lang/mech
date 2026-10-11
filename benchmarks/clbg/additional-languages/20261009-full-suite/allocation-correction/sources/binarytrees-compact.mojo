from std.sys import argv
from std.memory import Pointer
from std.origin import MutUntrackedOrigin
from common import write_string
from pool import Pool, initialize, terminate

@fieldwise_init
struct Node(ImplicitlyCopyable, Movable):
    var left: UInt64
    var right: UInt64

def build(depth: Int, pool: Pool) -> UInt64:
    var address = pool.allocate_bytes(16)
    var node = Pointer[Node, MutUntrackedOrigin](unsafe_from_address=Int(address))
    node.unsafe_write(Node(build(depth - 1, pool), build(depth - 1, pool)) if depth else Node(0, 0))
    return address

def check(address: UInt64) -> Int:
    var node = Pointer[Node, MutUntrackedOrigin](unsafe_from_address=Int(address))[]
    return 1 + check(node.left) + check(node.right) if node.left else 1

def main() raises:
    initialize()
    var depth = max(6, Int(argv()[1]))
    var stretch = Pool()
    var root = build(depth + 1, stretch)
    write_string("stretch tree of depth " + String(depth + 1) + "\t check: " + String(check(root)) + "\n")
    stretch.destroy()
    var long_lived = Pool()
    var long_root = build(depth, long_lived)
    var temporary = Pool()
    for d in range(4, depth + 1, 2):
        var iterations = 1 << (depth - d + 4)
        var total = 0
        for _ in range(iterations):
            total += check(build(d, temporary))
            temporary.clear()
        write_string(String(iterations) + "\t trees of depth " + String(d) + "\t check: " + String(total) + "\n")
    temporary.destroy()
    write_string("long lived tree of depth " + String(depth) + "\t check: " + String(check(long_root)) + "\n")
    long_lived.destroy()
    terminate()
