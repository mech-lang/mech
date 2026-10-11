"""General arena-allocation and thread optimizations; every tree node stays explicit."""
import json
from bootstrap import D, sha

ZIG = '''const std = @import("std");
const heap = std.heap.c_allocator;
const Node = struct { left: ?*Node, right: ?*Node };
fn capacity(depth: usize) usize {
    return ((@as(usize, 1) << @as(u6, @intCast(depth + 1))) - 1) * @sizeOf(Node);
}
fn build(allocator: std.mem.Allocator, depth: usize) !*Node {
    const node = try allocator.create(Node);
    node.* = .{ .left = null, .right = null };
    if (depth > 0) {
        node.left = try build(allocator, depth - 1);
        node.right = try build(allocator, depth - 1);
    }
    return node;
}
fn check(node: *const Node) usize {
    return 1 + (if (node.left) |left| check(left) else 0) + (if (node.right) |right| check(right) else 0);
}
fn batch(depth: usize, iterations: usize) !usize {
    const bytes = try heap.alloc(u8, capacity(depth));
    defer heap.free(bytes);
    var arena = std.heap.FixedBufferAllocator.init(bytes);
    var sum: usize = 0;
    for (0..iterations) |_| {
        sum += check(try build(arena.allocator(), depth));
        arena.reset();
    }
    return sum;
}
PARALLEL
pub fn main(init: std.process.Init) !void {
    var args = std.process.Args.Iterator.init(init.minimal.args);
    _ = args.skip();
    const n = try std.fmt.parseInt(usize, args.next() orelse "10", 10);
    const maximum = @max(6, n);
    var output_buffer: [65536]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output.interface.flush() catch unreachable;
    const stretch = try batch(maximum + 1, 1);
    try output.interface.print("stretch tree of depth {d}\\t check: {d}\\n", .{ maximum + 1, stretch });
    const long_bytes = try heap.alloc(u8, capacity(maximum));
    defer heap.free(long_bytes);
    var long_arena = std.heap.FixedBufferAllocator.init(long_bytes);
    const long_lived = try build(long_arena.allocator(), maximum);
    var depth: usize = 4;
    while (depth <= maximum) : (depth += 2) {
        const iterations = @as(usize, 1) << @as(u6, @intCast(maximum - depth + 4));
        const sum = try BATCH_CALL(depth, iterations);
        try output.interface.print("{d}\\t trees of depth {d}\\t check: {d}\\n", .{ iterations, depth, sum });
    }
    try output.interface.print("long lived tree of depth {d}\\t check: {d}\\n", .{ maximum, check(long_lived) });
}
'''
ZIG_PARALLEL = '''const Task = struct { depth: usize, iterations: usize, sum: usize = 0 };
fn worker(task: *Task) void {
    task.sum = batch(task.depth, task.iterations) catch unreachable;
}
fn parallelBatch(depth: usize, iterations: usize) !usize {
    const workers = @min(8, iterations);
    var tasks: [8]Task = undefined;
    var threads: [8]std.Thread = undefined;
    for (0..workers) |i| {
        tasks[i] = .{ .depth = depth, .iterations = iterations / workers + @intFromBool(i < iterations % workers) };
        threads[i] = try std.Thread.spawn(.{}, worker, .{&tasks[i]});
    }
    var sum: usize = 0;
    for (0..workers) |i| {
        threads[i].join();
        sum += tasks[i].sum;
    }
    return sum;
}
'''
ODIN = '''package main
import "core:fmt"
import "core:mem"
import "core:os"
import "core:strconv"
THREAD_IMPORT
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
PARALLEL
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 10
    maximum := max(6, n)
    stretch := batch(maximum+1, 1)
    fmt.printf("stretch tree of depth %d\\t check: %d\\n", maximum+1, stretch)
    long_bytes := make([]byte, capacity(maximum))
    defer delete(long_bytes)
    long_arena: mem.Arena
    mem.arena_init(&long_arena, long_bytes)
    long_lived := build(maximum, mem.arena_allocator(&long_arena))
    for depth := 4; depth <= maximum; depth += 2 {
        iterations := 1 << uint(maximum-depth+4)
        total := BATCH_CALL(depth, iterations)
        fmt.printf("%d\\t trees of depth %d\\t check: %d\\n", iterations, depth, total)
    }
    fmt.printf("long lived tree of depth %d\\t check: %d\\n", maximum, check(long_lived))
}
'''
ODIN_PARALLEL = '''Task :: struct { depth, iterations, total: int }
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
'''

def main():
    rows=[]
    for language, template, parallel in [('zig',ZIG,ZIG_PARALLEL),('odin',ODIN,ODIN_PARALLEL)]:
        for optimized in (False,True):
            name='binarytrees-'+language+'-arena'+('-parallel' if optimized else '-compact')
            source=template.replace('PARALLEL',parallel if optimized else '').replace('BATCH_CALL',
                ('parallelBatch' if language=='zig' else 'parallel_batch') if optimized else 'batch')
            if language=='odin':source=source.replace('THREAD_IMPORT','import "core:thread"' if optimized else '')
            path=D/'sources'/(name+'.'+language)
            path.write_text(source)
            rows.append(dict(key='binarytrees:'+name,id=name,benchmark='binarytrees',language=language,source=str(path),
                source_sha256=sha(path),source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html',
                local_adaptation=True,adaptations=['Locally authored full pointer-tree workload using the language’s general standard arena allocator'+(' and at most eight native workers' if optimized else '')+'. Every leaf/interior makes a separate equal-size Node allocation; both children are recursively allocated. Temporary arenas reset only after complete construction/check; stretch storage is released before a separately allocated long-lived tree, which remains live through all temporary work. Application allocator/thread plumbing counts toward LOC. No flat/indexed tree or native executor.'],libraries=[]))
    (D/'tree-extra.json').write_text(json.dumps(dict(programs=rows),indent=2)+'\n')
    print('PREPARED',len(rows),'faithful standard-arena candidates; compilation deferred until timing is idle.')

if __name__=='__main__':main()
