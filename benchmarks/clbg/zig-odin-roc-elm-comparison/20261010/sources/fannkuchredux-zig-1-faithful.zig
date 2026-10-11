// From https://github.com/tiehuis/zig-benchmarks-game/blob/master/src/fannkuch-redux.zig

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

var buffer: [1024]u8 = undefined;
var fixed_allocator = std.heap.FixedBufferAllocator.init(buffer[0..]);
var allocator = fixed_allocator.allocator();

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const stdout = Output{};
    const n = try get_n();

    const perm = try allocator.alloc(usize, n);
    const perm1 = try allocator.alloc(usize, n);
    const count = try allocator.alloc(usize, n);

    var max_flips_count: usize = 0;
    var perm_count: usize = 0;
    var checksum: isize = 0;

    for (perm1, 0..) |*e, i| {
        e.* = i;
    }

    var r = n;
    loop: {
        while (true) {
            while (r != 1) : (r -= 1) {
                count[r - 1] = r;
            }

            for (perm, 0..) |_, i| {
                perm[i] = perm1[i];
            }

            var flips_count: usize = 0;

            while (true) {
                const k = perm[0];
                if (k == 0) {
                    break;
                }

                const k2 = (k + 1) >> 1;
                var i: usize = 0;
                while (i < k2) : (i += 1) {
                    std.mem.swap(usize, &perm[i], &perm[k - i]);
                }
                flips_count += 1;
            }

            max_flips_count = @max(max_flips_count, flips_count);
            if (perm_count % 2 == 0) {
                checksum += @intCast(flips_count);
            } else {
                checksum -= @intCast(flips_count);
            }

            while (true) : (r += 1) {
                if (r == n) {
                    break :loop;
                }

                const perm0 = perm1[0];
                var i: usize = 0;
                while (i < r) {
                    const j = i + 1;
                    perm1[i] = perm1[j];
                    i = j;
                }

                perm1[r] = perm0;
                count[r] -= 1;

                if (count[r] > 0) {
                    break;
                }
            }

            perm_count += 1;
        }
    }

    try stdout.print("{d}\nPfannkuchen({d}) = {d}\n", .{ checksum, n, max_flips_count });
}

fn get_n() !usize {
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt(usize, args.next() orelse "10", 10);
}
