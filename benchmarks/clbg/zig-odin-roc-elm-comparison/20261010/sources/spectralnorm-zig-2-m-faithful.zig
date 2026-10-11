const std = @import("std");

var process_args: std.process.Args = undefined;
var process_io: std.Io = undefined;
var output_buffer: [65536]u8 = undefined;
var output_writer: std.Io.File.Writer = undefined;
const Output = struct {
    fn write(_: Output, bytes: []const u8) !usize {
        try output_writer.interface.writeAll(bytes);
        return bytes.len;
    }
    fn writeByte(self: Output, byte: u8) !void {
        _ = try self.write(&.{byte});
    }
    fn print(_: Output, comptime format: []const u8, args: anytype) !void {
        try output_writer.interface.print(format, args);
    }
};


var matrix_n: usize = 0;
const vec4 = @Vector(4, f64);
fn vec1to4(f: f64) vec4 {
    return @splat(f);
}

fn runInParallel(tasks: []std.Thread, len: usize, comptime f: anytype, args: anytype) !void {
    const len_per_task = @divTrunc(len, tasks.len + 1);
    for (tasks, 0..) |*task, i| {
        const first = len_per_task * i;
        const last = first + len_per_task;
        task.* = try std.Thread.spawn(.{}, f, .{ first, last } ++ args);
    }
    @call(.auto, f, .{ tasks.len * len_per_task, len } ++ args);
    for (tasks) |*task| task.join();
}

fn baseIdx(i: vec4) vec4 {
    @setFloatMode(.optimized);
    return i * (i + vec1to4(1)) * vec1to4(0.5) + vec1to4(1);
}

fn multAvGeneric(comptime transpose: bool, first: usize, dst: []vec4, src: []const vec4) void {
    @setFloatMode(.optimized);
    const srcVals = std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(src))[0..matrix_n];
    var ti = vec1to4(@as(f64, @floatFromInt(first * 4))) + if (transpose) vec4{ 1, 2, 3, 4 } else vec4{ 0, 1, 2, 3 };
    for (dst) |*res| {
        var idx = if (transpose) baseIdx(ti - vec1to4(1)) else baseIdx(ti) + ti;
        var sum = vec1to4(0);
        for (srcVals, 0..) |u, j| {
            sum += vec1to4(u) / idx;
            idx += ti + vec1to4(@as(f64, @floatFromInt(j + 1)));
        }
        res.* = sum;
        ti += vec1to4(4);
    }
}

fn multAv(first: usize, last: usize, dst: []vec4, src: []const vec4) void {
    return multAvGeneric(false, first, dst[first..last], src);
}

fn multAtv(first: usize, last: usize, dst: []vec4, src: []const vec4) void {
    return multAvGeneric(true, first, dst[first..last], src);
}

fn multAtAv(tasks: []std.Thread, dest: []vec4, src: []const vec4, temp: []vec4) !void {
    std.debug.assert(dest.len == src.len and src.len == temp.len);
    try runInParallel(tasks, dest.len, multAv, .{ temp, src });
    try runInParallel(tasks, dest.len, multAtv, .{ dest, temp });
}

fn setOnes(first: usize, last: usize, dst: []vec4) void {
    for (dst[first..last]) |*v| v.* = vec1to4(1);
}

fn aggregateResults(first: usize, last: usize, u: []const vec4, v: []const vec4, total_vbv: *f64, total_vv: *f64) void {
    const us = std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(u));
    const vs = std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(v));
    var vbv: f64 = 0;
    var vv: f64 = 0;
    for (first * 4..@min(last * 4, matrix_n)) |i| {
        vbv += us[i] * vs[i];
        vv += vs[i] * vs[i];
    }
    _ = @atomicRmw(f64, total_vbv, .Add, vbv, .seq_cst);
    _ = @atomicRmw(f64, total_vv, .Add, vv, .seq_cst);
}

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const n = try get_n();
    matrix_n = n;
    const len = (n + 3) / 4;

    const allocator = std.heap.c_allocator;
    const u = try allocator.alloc(vec4, len);
    defer allocator.free(u);
    const v = try allocator.alloc(vec4, len);
    defer allocator.free(v);
    const temp = try allocator.alloc(vec4, len);
    defer allocator.free(temp);

    const task_count = @min(8, try std.Thread.getCpuCount());
    const tasks = try allocator.alloc(std.Thread, task_count - 1);
    defer allocator.free(tasks);

    try runInParallel(tasks, u.len, setOnes, .{u});

    var times: usize = 0;
    while (times < 10) : (times += 1) {
        try multAtAv(tasks, v, u, temp);
        try multAtAv(tasks, u, v, temp);
    }

    var vbv: f64 = 0;
    var vv: f64 = 0;
    try runInParallel(tasks, u.len, aggregateResults, .{ u, v, &vbv, &vv });
    const res = std.math.sqrt(vbv / vv);

    const stdout = Output{};
    try stdout.print("{d:.9}\n", .{res});
}

fn get_n() !usize {
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt(usize, args.next() orelse "10", 10);
}
