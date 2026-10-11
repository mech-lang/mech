"""Complete Zig workload code using standard I/O, containers, GMP and PCRE2."""
import json
from bootstrap import D, sha

PREFIX = '''const std = @import("std");
const allocator = std.heap.c_allocator;
'''
INPUT = '''fn readAll(io: std.Io) ![]u8 {
    var bytes = std.array_list.Managed(u8).init(allocator);
    var buffer: [65536]u8 = undefined;
    while (true) {
        const got = std.Io.File.stdin().readStreaming(io, &.{&buffer}) catch |err| switch (err) { error.EndOfStream => break, else => return err };
        if (got == 0) break;
        try bytes.appendSlice(buffer[0..got]);
    }
    return bytes.toOwnedSlice();
}
'''

REVCOMP = '''fn complement(byte: u8) u8 {
    return switch (std.ascii.toUpper(byte)) {
        'A' => 'T', 'T', 'U' => 'A', 'C' => 'G', 'G' => 'C',
        'M' => 'K', 'R' => 'Y', 'W' => 'W', 'S' => 'S',
        'Y' => 'R', 'K' => 'M', 'V' => 'B', 'H' => 'D',
        'D' => 'H', 'B' => 'V', 'N' => 'N',
        else => unreachable,
    };
}
pub fn main(init: std.process.Init) !void {
    var output_buffer: [65536]u8 = undefined;
    var output = std.Io.File.stdout().writer(init.io, &output_buffer);
    const out = &output.interface;
    const input = try readAll(init.io);
    defer allocator.free(input);
    var record: usize = 0;
    while (record < input.len) {
        const header_end = std.mem.indexOfScalarPos(u8, input, record, '\\n') orelse return error.Header;
        try out.writeAll(input[record..header_end + 1]);
        const begin = header_end + 1;
        const end = std.mem.indexOfScalarPos(u8, input, begin, '>') orelse input.len;
        var count: usize = 0;
        for (input[begin..end]) |byte| {
            if (byte != '\\n' and byte != '\\r') {
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
                line[column] = '\\n';
                try out.writeAll(line[0..column + 1]);
                column = 0;
            }
        }
        record = end;
    }
    try out.flush();
}
'''

PCRE = '''const RegexCode = opaque {};
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
        JIT_LINE
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
    const cleaned = try substitute(raw, ">.*\\n|\\n", "");
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
        try out.print("{s} {d}\\n", .{pattern, count});
    }
    const substitutions = [_][2][]const u8{
        .{"tHa[Nt]", "<4>"}, .{"aND|caN|Ha[DS]|WaS", "<3>"},
        .{"a[NSt]|BY", "<2>"}, .{"<[^>]*>", "|"}, .{"\\\\|[^|][^|]*\\\\|", "-"},
    };
    var replaced = cleaned;
    for (substitutions) |replacement| {
        const next = try substitute(replaced, replacement[0], replacement[1]);
        if (replaced.ptr != cleaned.ptr) allocator.free(replaced);
        replaced = next;
    }
    try out.print("\\n{d}\\n{d}\\n{d}\\n", .{raw.len, cleaned.len, replaced.len});
    allocator.free(replaced);
    allocator.free(cleaned);
    allocator.free(raw);
    try out.flush();
}
'''

GMP = '''const Integer = extern struct { allocated: c_int, size: c_int, limbs: ?[*]u64 };
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
                try out.print("{s}\\t:{d}\\n", .{line, produced});
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
'''

def main():
    rows = []
    def emit(benchmark, variant, source, libraries=None):
        identifier = benchmark + '-zig-' + variant
        path = D / 'sources' / (identifier + '.zig')
        path.write_text(PREFIX + source)
        rows.append(dict(key=benchmark + ':' + identifier, id=identifier, benchmark=benchmark, language='zig',
            source=str(path), source_sha256=sha(path), local_adaptation=True, adaptations=['Locally authored faithful workload using only general language libraries; all application logic and bindings count toward LOC.'],
            source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/' + benchmark + '.html',
            uses_gmp='gmp' in (libraries or []), libraries=libraries or []))
    emit('revcomp', 'compact', INPUT + REVCOMP)
    table = '''const complement_table: [256]u8 = table: {
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
'''
    from prepare_published import replace_function
    fast = replace_function(REVCOMP, 'fn complement(', table)
    emit('revcomp', 'table', INPUT + fast)
    emit('regexredux', 'compact-pcre2', INPUT + PCRE.replace('JIT_LINE', ''))
    emit('regexredux', 'jit-pcre2', INPUT + PCRE.replace('JIT_LINE', '_ = pcre2_jit_compile_8(code, 1);'))
    emit('pidigits', 'gmp', GMP, ['gmp'])
    (D / 'written-zig.json').write_text(json.dumps(dict(programs=rows), indent=2) + '\n')
    print('WROTE ZIG', len(rows), flush=True)

if __name__ == '__main__': main()
