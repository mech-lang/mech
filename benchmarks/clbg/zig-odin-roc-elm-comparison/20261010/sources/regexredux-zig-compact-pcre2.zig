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
const RegexCode = opaque {};
const MatchData = opaque {};
extern "c" fn pcre2_compile_8([*]const u8, usize, u32, *c_int, *usize, ?*anyopaque) ?*RegexCode;
extern "c" fn pcre2_code_free_8(*RegexCode) void;
extern "c" fn pcre2_match_data_create_from_pattern_8(*RegexCode, ?*anyopaque) ?*MatchData;
extern "c" fn pcre2_match_data_free_8(*MatchData) void;
extern "c" fn pcre2_match_8(*RegexCode, [*]const u8, usize, usize, u32, *MatchData, ?*anyopaque) c_int;
extern "c" fn pcre2_get_ovector_pointer_8(*MatchData) [*]usize;
extern "c" fn pcre2_jit_compile_8(*RegexCode, u32) c_int;
const Regex = struct {
    code: *RegexCode,
    match_data: *MatchData,
    fn init(pattern: []const u8, caseless: bool) !Regex {
        var error_code: c_int = 0;
        var error_offset: usize = 0;
        const code = pcre2_compile_8(pattern.ptr, pattern.len, if (caseless) 8 else 0, &error_code, &error_offset, null) orelse return error.RegexCompile;
        
        return .{.code = code, .match_data = pcre2_match_data_create_from_pattern_8(code, null) orelse return error.MatchAllocation};
    }
    fn deinit(self: Regex) void {
        pcre2_match_data_free_8(self.match_data);
        pcre2_code_free_8(self.code);
    }
    fn find(self: Regex, input: []const u8, offset: usize) !?[2]usize {
        const status = pcre2_match_8(self.code, input.ptr, input.len, offset, 0, self.match_data, null);
        if (status == -1) return null;
        if (status < 0) return error.RegexMatch;
        const range = pcre2_get_ovector_pointer_8(self.match_data);
        return .{range[0], range[1]};
    }
};
fn substitute(input: []const u8, pattern: []const u8, replacement: []const u8) ![]u8 {
    const regex = try Regex.init(pattern, false);
    defer regex.deinit();
    var output = std.array_list.Managed(u8).init(allocator);
    var offset: usize = 0;
    while (try regex.find(input, offset)) |range| {
        try output.appendSlice(input[offset..range[0]]);
        try output.appendSlice(replacement);
        offset = range[1];
    }
    try output.appendSlice(input[offset..]);
    return output.toOwnedSlice();
}
pub fn main(init: std.process.Init) !void {
    var output_buffer: [4096]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    const out = &output.interface;
    const raw = try readAll(init.io);
    const cleaned = try substitute(raw, ">.*\n|\n", "");
    const patterns = [_][]const u8{
        "agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct",
    };
    for (patterns) |pattern| {
        const regex = try Regex.init(pattern, true);
        defer regex.deinit();
        var offset: usize = 0;
        var count: usize = 0;
        while (try regex.find(cleaned, offset)) |range| {
            count += 1;
            offset = range[1];
        }
        try out.print("{s} {d}\n", .{pattern, count});
    }
    const substitutions = [_][2][]const u8{
        .{"tHa[Nt]", "<4>"}, .{"aND|caN|Ha[DS]|WaS", "<3>"},
        .{"a[NSt]|BY", "<2>"}, .{"<[^>]*>", "|"}, .{"\\|[^|][^|]*\\|", "-"},
    };
    var replaced = cleaned;
    for (substitutions) |replacement| {
        const next = try substitute(replaced, replacement[0], replacement[1]);
        if (replaced.ptr != cleaned.ptr) allocator.free(replaced);
        replaced = next;
    }
    try out.print("\n{d}\n{d}\n{d}\n", .{raw.len, cleaned.len, replaced.len});
    allocator.free(replaced);
    allocator.free(cleaned);
    allocator.free(raw);
    try out.flush();
}
