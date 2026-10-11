const std = @import("std");
const allocator = std.heap.c_allocator;
fn readAll(io: std.Io) ![]u8 {
    var bytes = std.array_list.Managed(u8).init(allocator);
    var buffer: [65536]u8 = undefined;
    while (true) {
        const got = std.Io.File.stdin().readStreaming(io, &.{&buffer}) catch |err| switch (err) { error.EndOfStream => break, else => return err };
        if (got == 0) break;
        try bytes.appendSlice(buffer[0..got]);
    }
    return bytes.toOwnedSlice();
}
const complement_table: [256]u8 = table: {
    @setEvalBranchQuota(10000);
    var entries: [256]u8 = @splat(0);
    const letters = "ATUCGMRWSYKVHDBN";
    const values = "TAAGCKYWSRMBDHVN";
    for (letters, values) |letter, value| {
        entries[letter] = value;
        entries[std.ascii.toLower(letter)] = value;
    }
    break :table entries;
};
fn complement(byte: u8) u8 {
    return complement_table[byte];
}

pub fn main(init: std.process.Init) !void {
    var output_buffer: [65536]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    const out = &output.interface;
    const input = try readAll(init.io);
    defer allocator.free(input);
    var record: usize = 0;
    while (record < input.len) {
        const header_end = std.mem.indexOfScalarPos(u8, input, record, '\n') orelse return error.Header;
        try out.writeAll(input[record..header_end + 1]);
        const begin = header_end + 1;
        const end = std.mem.indexOfScalarPos(u8, input, begin, '>') orelse input.len;
        var count: usize = 0;
        for (input[begin..end]) |byte| {
            if (byte != '\n' and byte != '\r') {
                input[begin + count] = byte;
                count += 1;
            }
        }
        var line: [61]u8 = undefined;
        var column: usize = 0;
        while (count > 0) {
            count -= 1;
            line[column] = complement(input[begin + count]);
            column += 1;
            if (column == 60 or count == 0) {
                line[column] = '\n';
                try out.writeAll(line[0..column + 1]);
                column = 0;
            }
        }
        record = end;
    }
    try out.flush();
}
