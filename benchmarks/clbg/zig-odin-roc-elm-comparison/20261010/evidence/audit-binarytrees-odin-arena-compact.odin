package main
import "core:fmt"
import "core:sync"
allocated: int
import "core:mem"
import "core:os"
import "core:strconv"

Node :: struct { left, right: ^Node }
capacity :: proc(depth: int) -> int {
    return ((1 << uint(depth+1))-1)*size_of(Node)
}
build :: proc(depth: int, allocator: mem.Allocator) -> ^Node {
    node := new(Node, allocator)
    _ = sync.atomic_add(&allocated, 1)
    assert(node != nil)
    if depth > 0 {
        node.left = build(depth-1, allocator)
        node.right = build(depth-1, allocator)
        if node.left == node.right { panic("Shared children") }
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
        total := batch(depth, iterations)
        fmt.printf("%d\t trees of depth %d\t check: %d\n", iterations, depth, total)
    }
    fmt.printf("long lived tree of depth %d\t check: %d\n", maximum, check(long_lived))
    fmt.eprintf("AUDIT %d %d\n", size_of(Node), sync.atomic_load(&allocated))
}
