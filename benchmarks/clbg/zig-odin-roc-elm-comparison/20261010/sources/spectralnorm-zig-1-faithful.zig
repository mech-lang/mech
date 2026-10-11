// From https://github.com/tiehuis/zig-benchmarks-game/blob/master/src/spectral-norm.zig

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


const global_allocator = std.heap.c_allocator;

fn eval_a(i: usize, j: usize) f64 {
    return 1.0 / @as(f64, @floatFromInt((i + j) * (i + j + 1) / 2 + i + 1));
}

fn eval_a_times_u(comptime transpose: bool, au: []f64, u: []const f64) void {
    for (au) |*e| {
        e.* = 0;
    }

    var i: usize = 0;
    while (i < au.len) : (i += 1) {
        var j: usize = 0;
        var a: f64 = 0.0;
        while (j < au.len) : (j += 1) {
            if (transpose) {
                a += eval_a(j, i) * u[j];
            } else {
                a += eval_a(i, j) * u[j];
            }
        }
        au[i] = a;
    }
}

fn eval_ata_times_u(atau: []f64, u: []const f64, scratch: []f64) void {
    std.debug.assert(atau.len == u.len and u.len == scratch.len);

    eval_a_times_u(false, scratch, u);
    eval_a_times_u(true, atau, scratch);
}

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const stdout = Output{};
    const n = try get_n();

    const u = try global_allocator.alloc(f64, n);
    const v = try global_allocator.alloc(f64, n);
    const scratch = try global_allocator.alloc(f64, n);

    for (u) |*e| {
        e.* = 1;
    }

    var i: usize = 0;
    while (i < 10) : (i += 1) {
        eval_ata_times_u(v, u, scratch);
        eval_ata_times_u(u, v, scratch);
    }

    var vbv: f64 = 0;
    var vv: f64 = 0;

    var j: usize = 0;
    while (j < n) : (j += 1) {
        vbv += u[j] * v[j];
        vv += v[j] * v[j];
    }

    try stdout.print("{d:.9}\n", .{std.math.sqrt(vbv / vv)});
}

fn get_n() !usize {
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt(usize, args.next() orelse "10", 10);
}
