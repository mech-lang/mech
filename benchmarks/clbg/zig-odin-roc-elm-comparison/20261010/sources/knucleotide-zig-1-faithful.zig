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
const stdout = Output{};

const Code = struct {
    data: u64,

    pub inline fn encodeByte(c: u8) u2 {
        return switch (c) { 'A' => 0, 'C' => 1, 'G' => 2, 'T' => 3, else => unreachable };
    }

    pub inline fn makeMask(frame: usize) u64 {
        return (@as(u64, 1) << @as(u6, @intCast(2 * frame))) - 1;
    }

    pub inline fn push(self: *Code, c: u8, mask: u64) void {
        self.data = ((self.data << 2) | c) & mask;
    }

    pub fn fromStr(s: []const u8) Code {
        const mask = Code.makeMask(s.len);
        var res = Code{ .data = 0 };
        for (s) |c| {
            res.push(Code.encodeByte(c), mask);
        }
        return res;
    }

    pub fn toString(self: Code, frame: usize) ![]const u8 {
        var result = std.array_list.Managed(u8).init(global_allocator);
        var code = self.data;
        var i: usize = 0;
        while (i < frame) : (i += 1) {
            const c: u8 = switch (@as(u8, @truncate(code)) & 0b11) {
                Code.encodeByte('A') => 'A',
                Code.encodeByte('T') => 'T',
                Code.encodeByte('G') => 'G',
                Code.encodeByte('C') => 'C',
                else => unreachable,
            };
            try result.append(c);
            code >>= 2;
        }
        std.mem.reverse(u8, result.items);
        return result.toOwnedSlice();
    }
};

pub fn readInput() ![]const u8 {
    var raw = std.array_list.Managed(u8).init(global_allocator);
    var chunk: [65536]u8 = undefined;
    while (true) {
        const got = std.Io.File.stdin().readStreaming(process_io, &.{&chunk}) catch |err| switch (err) { error.EndOfStream => break, else => return err };
        if (got == 0) break;
        try raw.appendSlice(chunk[0..got]);
    }
    const start = std.mem.indexOf(u8, raw.items, ">THREE") orelse return error.MissingRecord;
    const header_end = std.mem.indexOfScalarPos(u8, raw.items, start, '\n') orelse return error.MissingHeaderEnd;
    var used: usize = 0;
    for (raw.items[header_end + 1..]) |byte| {
        if (byte == '>') break;
        if (byte == '\n' or byte == '\r') continue;
        raw.items[used] = Code.encodeByte(std.ascii.toUpper(byte));
        used += 1;
    }
    return raw.items[0..used];
}

const CodeContext = struct {
    pub fn eql(_: CodeContext, a: Code, b: Code) bool { return a.data == b.data; }
    pub fn hash(_: CodeContext, c: Code) u64 { return c.data ^ (c.data >> 7); }
};

const Map = std.HashMapUnmanaged(Code, u32, CodeContext, 45);

const Iter = struct {
    i: usize = 0,
    input: []const u8,
    code: Code,
    mask: u64,

    pub fn init(input: []const u8, frame: usize) Iter {
        const mask = Code.makeMask(frame);
        var code = Code{ .data = 0 };
        for (input[0 .. frame - 1]) |c| code.push(c, mask);
        return .{
            .input = input[frame - 1 ..],
            .code = code,
            .mask = mask,
        };
    }

    pub fn next(self: *Iter) ?Code {
        if (self.i >= self.input.len) return null;
        defer self.i += 1;
        const c = self.input[self.i];
        Code.push(&self.code, c, self.mask);
        return self.code;
    }
};

fn genMap(seq: []const u8, n: usize, map: *Map) !void {
    map.clearAndFree(global_allocator);
    var iter = Iter.init(seq, n);
    while (iter.next()) |code| {
        const gop = try map.getOrPut(global_allocator, code);
        if (!gop.found_existing) gop.value_ptr.* = 0;
        gop.value_ptr.* += 1;
    }
}

const CountCode = struct {
    count: u64,
    code: Code,
    pub fn asc(_: void, a: CountCode, b: CountCode) bool {
        const order = std.math.order(a.count, b.count);
        return order == .lt or (order == .eq and b.code.data < a.code.data);
    }
};

fn printMap(self: usize, map: Map) !void {
    var v = std.array_list.Managed(CountCode).init(global_allocator);
    defer v.deinit();
    var iter = map.iterator();
    var total: u64 = 0;
    while (iter.next()) |it| {
        const count = it.value_ptr.*;
        total += count;
        try v.append(.{ .count = count, .code = it.key_ptr.* });
    }

    std.mem.sort(CountCode, v.items, {}, comptime CountCode.asc);
    var i = v.items.len - 1;
    while (true) : (i -= 1) {
        const cc = v.items[i];
        try stdout.print("{!s} {d:.3}\n", .{
            cc.code.toString(self),
            @as(f64, @floatFromInt(cc.count)) / @as(f64, @floatFromInt(total)) * 100.0,
        });
        if (i == 0) break;
    }
    try stdout.print("\n", .{});
}

fn printOcc(s: []const u8, map: *Map) !void {
    const count = if (map.get(Code.fromStr(s))) |x| x else 0;
    try stdout.print("{}\t{s}\n", .{ count, s });
}

pub fn main(init: std.process.Init) !void {
    process_args = init.minimal.args;
    process_io = init.io;
    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);
    defer output_writer.interface.flush() catch unreachable;
    const occs = [_][]const u8{
        "GGT",
        "GGTA",
        "GGTATT",
        "GGTATTTTAATT",
        "GGTATTTTAATTTATAGT",
    };
    const input = try readInput();
    var map: Map = .{};
    try genMap(input, 1, &map);
    try printMap(1, map);
    try genMap(input, 2, &map);
    try printMap(2, map);

    for (occs) |occ| {
        try genMap(input, occ.len, &map);
        try printOcc(occ, &map);
    }
}
