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

const bigint = std.math.big.int;
const global_allocator = std.heap.c_allocator;

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const stdout = Output{};
    const n = try get_n();

    const one = (try bigint.Managed.initSet(global_allocator, 1));
    const two = (try bigint.Managed.initSet(global_allocator, 2));
    const ten = (try bigint.Managed.initSet(global_allocator, 10));

    var k = try bigint.Managed.initSet(global_allocator, 1);
    var n1 = try bigint.Managed.initSet(global_allocator, 4);
    var n2 = try bigint.Managed.initSet(global_allocator, 3);
    var d = try bigint.Managed.initSet(global_allocator, 1);
    var tmp = try bigint.Managed.init(global_allocator);
    var tmp2 = try bigint.Managed.init(global_allocator);
    var v = try bigint.Managed.init(global_allocator);
    var u = try bigint.Managed.init(global_allocator);
    var w = try bigint.Managed.init(global_allocator);

    var digits_printed: usize = 0;
    var lbuf: [10]std.math.big.Limb = undefined;
    var sb: [10]u8 = undefined;
    while (true) {
        // u = &n1 / &d;
        try bigint.Managed.divFloor(&u, &tmp, &n1, &d);
        // v = &n2 / &d;
        try bigint.Managed.divFloor(&v, &tmp, &n2, &d);
        // if u == v
        if (bigint.Managed.eql(u, v)) {
            const rem = @rem(digits_printed, 10);
            _ = u.toConst().toString(sb[rem..], 10, .lower, &lbuf);
            digits_printed += 1;
            if (rem == 9)
                try stdout.print("{s}\t:{d}\n", .{ sb, digits_printed });

            if (digits_printed >= n) {
                if (rem != 9) {
                    @memset(sb[rem + 1 ..], ' ');
                    try stdout.print("{s}\t:{d}\n", .{ sb, digits_printed });
                }
                break;
            }
            // let to_minus = &u * &ten * &d;
            try bigint.Managed.mul(&tmp, &u, &d);
            try bigint.Managed.mul(&tmp2, &tmp, &ten);
            // n1 = &n1 * &ten - &to_minus;
            try bigint.Managed.mul(&tmp, &n1, &ten);
            try bigint.Managed.sub(&n1, &tmp, &tmp2);
            // n2 = &n2 * &ten - &to_minus;
            try bigint.Managed.mul(&tmp, &n2, &ten);
            try bigint.Managed.sub(&n2, &tmp, &tmp2);
        } else {
            // let k2 = &k * &two;
            try bigint.Managed.mul(&tmp2, &k, &two);
            // u = &n1 * (&k2 - &one);
            try bigint.Managed.sub(&tmp, &tmp2, &one);
            try bigint.Managed.mul(&u, &tmp, &n1);
            // v = &n2 * &two;
            try bigint.Managed.mul(&v, &n2, &two);
            // w = &n1 * (&k - &one);
            try bigint.Managed.sub(&tmp, &k, &one);
            try bigint.Managed.mul(&w, &tmp, &n1);
            // n1 = &u + &v;
            try bigint.Managed.add(&n1, &u, &v);
            // u = &n2 * (&k + &two);
            try bigint.Managed.add(&tmp, &k, &two);
            try bigint.Managed.mul(&u, &tmp, &n2);
            // n2 = &w + &u;
            try bigint.Managed.add(&n2, &w, &u);
            // d = &d * (&k2 + &one);
            try bigint.Managed.add(&tmp, &tmp2, &one);
            try bigint.Managed.mul(&d, &tmp, &d);
            // k = &k + &one;
            try bigint.Managed.add(&k, &k, &one);
        }
    }
}

fn get_n() !i32 {
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt(i32, args.next() orelse "10", 10);
}
