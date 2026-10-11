const std = @import("std");
const allocator = std.heap.c_allocator;
const Integer = extern struct { allocated: c_int, size: c_int, limbs: ?[*]u64 };
extern "c" fn __gmpz_init(*Integer) void;
extern "c" fn __gmpz_set_ui(*Integer, c_ulong) void;
extern "c" fn __gmpz_mul_ui(*Integer, *const Integer, c_ulong) void;
extern "c" fn __gmpz_add(*Integer, *const Integer, *const Integer) void;
extern "c" fn __gmpz_sub(*Integer, *const Integer, *const Integer) void;
extern "c" fn __gmpz_fdiv_q(*Integer, *const Integer, *const Integer) void;
extern "c" fn __gmpz_cmp(*const Integer, *const Integer) c_int;
extern "c" fn __gmpz_get_ui(*const Integer) c_ulong;
extern "c" fn __gmpz_clear(*Integer) void;
pub fn main(init: std.process.Init) !void {
    var args = std.process.Args.Iterator.init(init.minimal.args);
    _ = args.skip();
    const n = try std.fmt.parseInt(usize, args.next() orelse "100", 10);
    var output_buffer: [4096]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    const out = &output.interface;
    var values: [7]Integer = undefined;
    for (&values) |*value| __gmpz_init(value);
    defer for (&values) |*value| __gmpz_clear(value);
    const q = &values[0];
    const r = &values[1];
    const t = &values[2];
    const a = &values[3];
    const b = &values[4];
    const first = &values[5];
    const second = &values[6];
    __gmpz_set_ui(q, 1);
    __gmpz_set_ui(t, 1);
    var k: usize = 0;
    var produced: usize = 0;
    var line: [10]u8 = undefined;
    while (produced < n) {
        __gmpz_mul_ui(a, q, 3);
        __gmpz_add(a, a, r);
        __gmpz_fdiv_q(first, a, t);
        __gmpz_mul_ui(b, q, 4);
        __gmpz_add(b, b, r);
        __gmpz_fdiv_q(second, b, t);
        if (__gmpz_cmp(first, second) == 0) {
            const digit = __gmpz_get_ui(first);
            line[produced % 10] = @as(u8, @intCast(digit)) + '0';
            produced += 1;
            if (produced % 10 == 0 or produced == n) {
                const used = if (produced % 10 == 0) 10 else produced % 10;
                @memset(line[used..], ' ');
                try out.print("{s}\t:{d}\n", .{line, produced});
            }
            __gmpz_mul_ui(a, t, digit);
            __gmpz_sub(r, r, a);
            __gmpz_mul_ui(r, r, 10);
            __gmpz_mul_ui(q, q, 10);
        } else {
            k += 1;
            const multiplier = 2 * k + 1;
            __gmpz_mul_ui(a, q, 2);
            __gmpz_add(r, r, a);
            __gmpz_mul_ui(r, r, multiplier);
            __gmpz_mul_ui(q, q, k);
            __gmpz_mul_ui(t, t, multiplier);
        }
    }
    try out.flush();
}
