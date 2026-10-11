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

const ArrayList = std.array_list.Managed;
const md5 = std.crypto.hash.Md5;

const VEC_SIZE = 8;
const Vec = @Vector(VEC_SIZE, f64);

const global_allocator = std.heap.c_allocator;

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const n = try get_n();
    const size = (n + VEC_SIZE - 1) / VEC_SIZE * VEC_SIZE;
    const chunk_size = size / VEC_SIZE;
    const inv = 2.0 / @as(f64, @floatFromInt(size));
    var xloc = ArrayList(Vec).init(global_allocator);
    try xloc.ensureTotalCapacityPrecise(chunk_size);
    var i: usize = 0;
    while (i < chunk_size) : (i += 1) {
        const offset = i * VEC_SIZE;
        const v = Vec{
            init_xloc(offset, inv),
            init_xloc(offset + 1, inv),
            init_xloc(offset + 2, inv),
            init_xloc(offset + 3, inv),
            init_xloc(offset + 4, inv),
            init_xloc(offset + 5, inv),
            init_xloc(offset + 6, inv),
            init_xloc(offset + 7, inv),
        };
        try xloc.append(v);
    }

    const stdout = Output{};
    try stdout.print("P4\n{d} {d}\n", .{ size, size });

    var pixels = ArrayList(u8).init(global_allocator);
    try pixels.ensureTotalCapacityPrecise(size * chunk_size);
    var y: usize = 0;
    while (y < size) : (y += 1) {
        const ci = @as(f64, @floatFromInt(y)) * inv - 1.0;
        var x: usize = 0;
        while (x < chunk_size) : (x += 1) {
            const r = mbrot8(xloc.items[x], ci);
            try pixels.append(r);
        }
    }

    // try stdout.print("{}\n", .{pixels});
    _ = try stdout.write(pixels.items);
}

fn mbrot8(cr: Vec, civ: f64) u8 {
    const ci = @as(Vec, @splat(civ));
    const zero: f64 = 0.0;
    var zr = @as(Vec, @splat(zero));
    var zi = @as(Vec, @splat(zero));
    var tr = @as(Vec, @splat(zero));
    var ti = @as(Vec, @splat(zero));
    var absz = @as(Vec, @splat(zero));

    var _i: u8 = 0;
    while (_i < 10) : (_i += 1) {
        var _j: u8 = 0;
        while (_j < 5) : (_j += 1) {
            zi = (zr + zr) * zi + ci;
            zr = tr - ti + cr;
            tr = zr * zr;
            ti = zi * zi;
        }
        absz = tr + ti;
        if (!@reduce(.Or, absz <= @as(Vec, @splat(4.0)))) {
            return 0;
        }
    }
    var accu: u8 = 0;
    inline for (0..VEC_SIZE) |i| {
        if (absz[i] <= 4.0) {
            const lhs: u8 = 0x80;
            accu |= (lhs >> @as(u3, @intCast(i)));
        }
    }
    return accu;
}

fn init_xloc(i: usize, inv: f64) f64 {
    return @as(f64, @floatFromInt(i)) * inv - 1.5;
}

fn get_n() !usize {
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt(usize, args.next() orelse "10", 10);
}
