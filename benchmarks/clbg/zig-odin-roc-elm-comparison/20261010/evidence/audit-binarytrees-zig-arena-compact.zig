const std = @import("std");
const heap = std.heap.c_allocator;
var allocated = std.atomic.Value(usize).init(0);
const Node = struct { left: ?*Node, right: ?*Node };
fn capacity(depth: usize) usize {
    return ((@as(usize, 1) << @as(u6, @intCast(depth + 1))) - 1) * @sizeOf(Node);
}
fn build(allocator: std.mem.Allocator, depth: usize) !*Node {
    const node = try allocator.create(Node);
    _ = allocated.fetchAdd(1, .monotonic);
    node.* = .{ .left = null, .right = null };
    if (depth > 0) {
        node.left = try build(allocator, depth - 1);
        node.right = try build(allocator, depth - 1);
        if (node.left == node.right) return error.SharedChildren;
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

pub fn main(init: std.process.Init) !void {
    var args = std.process.Args.Iterator.init(init.minimal.args);
    _ = args.skip();
    const n = try std.fmt.parseInt(usize, args.next() orelse "10", 10);
    const maximum = @max(6, n);
    var output_buffer: [65536]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output.interface.flush() catch unreachable;
    const stretch = try batch(maximum + 1, 1);
    try output.interface.print("stretch tree of depth {d}\t check: {d}\n", .{ maximum + 1, stretch });
    const long_bytes = try heap.alloc(u8, capacity(maximum));
    defer heap.free(long_bytes);
    var long_arena = std.heap.FixedBufferAllocator.init(long_bytes);
    const long_lived = try build(long_arena.allocator(), maximum);
    var depth: usize = 4;
    while (depth <= maximum) : (depth += 2) {
        const iterations = @as(usize, 1) << @as(u6, @intCast(maximum - depth + 4));
        const sum = try batch(depth, iterations);
        try output.interface.print("{d}\t trees of depth {d}\t check: {d}\n", .{ iterations, depth, sum });
    }
    try output.interface.print("long lived tree of depth {d}\t check: {d}\n", .{ maximum, check(long_lived) });
    std.debug.print("AUDIT {d} {d}\n", .{ @sizeOf(Node), allocated.load(.monotonic) });
}
