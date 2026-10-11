package main
import "core:fmt"
import "core:mem"
import "core:os"
import "core:strconv"
import "core:thread"
Node :: struct { left, right: ^Node }
capacity :: proc(depth: int) -> int {
    return ((1 << uint(depth+1))-1)*size_of(Node)
}
build :: proc(depth: int, allocator: mem.Allocator) -> ^Node {
    node := new(Node, allocator)
    assert(node != nil)
    if depth > 0 {
        node.left = build(depth-1, allocator)
        node.right = build(depth-1, allocator)
    }
    return node
}
check :: proc(node: ^Node) -> int {
    if node == nil { return 0 }
    return 1+check(node.left)+check(node.right)
}
batch :: proc(depth, iterations: int) -> int {
    bytes := make([]byte, capacity(depth))
    defer delete(bytes)
    arena: mem.Arena
    mem.arena_init(&arena, bytes)
    total := 0
    for _ in 0..<iterations {
        total += check(build(depth, mem.arena_allocator(&arena)))
        mem.arena_free_all(&arena)
    }
    return total
}
Task :: struct { depth, iterations, total: int }
worker :: proc(t: ^thread.Thread) {
    task := cast(^Task)t.data
    task.total = batch(task.depth, task.iterations)
}
parallel_batch :: proc(depth, iterations: int) -> int {
    workers := min(8, iterations)
    tasks: [8]Task
    threads: [8]^thread.Thread
    for i in 0..<workers {
        tasks[i] = Task{depth, iterations/workers + int(i < iterations%workers), 0}
        threads[i] = thread.create(worker)
        threads[i].data = &tasks[i]
        thread.start(threads[i])
    }
    total := 0
    for i in 0..<workers {
        thread.join(threads[i])
        total += tasks[i].total
        thread.destroy(threads[i])
    }
    return total
}

main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 10
    maximum := max(6, n)
    stretch := batch(maximum+1, 1)
    fmt.printf("stretch tree of depth %d\t check: %d\n", maximum+1, stretch)
    long_bytes := make([]byte, capacity(maximum))
    defer delete(long_bytes)
    long_arena: mem.Arena
    mem.arena_init(&long_arena, long_bytes)
    long_lived := build(maximum, mem.arena_allocator(&long_arena))
    for depth := 4; depth <= maximum; depth += 2 {
        iterations := 1 << uint(maximum-depth+4)
        total := parallel_batch(depth, iterations)
        fmt.printf("%d\t trees of depth %d\t check: %d\n", iterations, depth, total)
    }
    fmt.printf("long lived tree of depth %d\t check: %d\n", maximum, check(long_lived))
}
