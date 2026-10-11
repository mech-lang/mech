"""Preserve algorithms while porting published Zig/Odin sources to this cohort."""
import json
from pathlib import Path
import re
from bootstrap import D, sha

ZIG_IO = '''
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
'''

ZIG_INPUT = '''pub fn readInput() ![]const u8 {
    var raw = std.array_list.Managed(u8).init(global_allocator);
    var chunk: [65536]u8 = undefined;
    while (true) {
        const got = std.Io.File.stdin().readStreaming(process_io, &.{&chunk}) catch |err| switch (err) { error.EndOfStream => break, else => return err };
        if (got == 0) break;
        try raw.appendSlice(chunk[0..got]);
    }
    const start = std.mem.indexOf(u8, raw.items, ">THREE") orelse return error.MissingRecord;
    const header_end = std.mem.indexOfScalarPos(u8, raw.items, start, '\\n') orelse return error.MissingHeaderEnd;
    var used: usize = 0;
    for (raw.items[header_end + 1..]) |byte| {
        if (byte == '>') break;
        if (byte == '\\n' or byte == '\\r') continue;
        raw.items[used] = Code.encodeByte(std.ascii.toUpper(byte));
        used += 1;
    }
    return raw.items[0..used];
}'''

def function_span(source, marker):
    start = source.index(marker)
    opening = source.index('{', start)
    depth = 1
    end = opening + 1
    while depth:
        if source[end] == '{': depth += 1
        if source[end] == '}': depth -= 1
        end += 1
    return start, end

def replace_function(source, marker, replacement):
    start, end = function_span(source, marker)
    return source[:start] + replacement + source[end:]

def zig(source, benchmark, identifier):
    changes = ['Port command-line access and buffered stdin/stdout to Zig 0.17 standard Io interfaces; all application I/O helper code counts toward LOC.']
    source = source.replace('const std = @import("std");', 'const std = @import("std");\n' + ZIG_IO, 1)
    source = source.replace('pub fn main() !void {', 'pub fn main(init: std.process.Init) !void {\n    process_args = init.minimal.args;\n    process_io = init.io;\n    output_writer = std.Io.File.stdout().writer(init.io, &output_buffer);\n    defer output_writer.interface.flush() catch unreachable;')
    source = source.replace('std.io.getStdOut().writer()', 'Output{}')
    source = source.replace('std.process.args()', 'std.process.Args.Iterator.init(process_args)')
    source = source.replace('std.ArrayList', 'std.array_list.Managed').replace('.Vector.len', '.vector.len')
    source = source.replace('try std.Thread.getCpuCount()', '@min(8, try std.Thread.getCpuCount())')
    source = source.replace('fixed_allocator.allocator;', 'fixed_allocator.allocator();')
    if 'fn get_n()' in source:
        return_type = re.search(r'fn get_n\(\) !([a-z0-9]+)', source).group(1)
        source = replace_function(source, 'fn get_n()', f'''fn get_n() !{return_type} {{
    var args = std.process.Args.Iterator.init(process_args);
    _ = args.skip();
    return std.fmt.parseInt({return_type}, args.next() orelse "10", 10);
}}''')
    if benchmark == 'fasta':
        source = source.replace('var buffered_stdout = std.io.bufferedWriter(Output{});\n    defer buffered_stdout.flush() catch unreachable;\n    const stdout = buffered_stdout.writer();', 'const stdout = Output{};')
        source = source.replace('cum_prob_total[i] = cum_prob * im;', 'cum_prob_total[i] = cum_prob;').replace('nextRandom(im)', 'nextRandom(1.0)')
        changes.append('Use unscaled binary64 cumulative probabilities and a fresh direct binary64 LCG division for every random symbol, instead of integer-scaled thresholds.')
    if benchmark == 'spectralnorm':
        if identifier == '1':
            source = source.replace('vbv += u[i] * v[i];', 'vbv += u[j] * v[j];').replace('vv += v[i] * v[i];', 'vv += v[j] * v[j];')
            changes.append('Correct the final dot-product index from the stale loop variable i to j, so every final element participates.')
        else:
            source = source.replace('const vec4 =', 'var matrix_n: usize = 0;\nconst vec4 =', 1)
            source = source.replace('const n = try get_n();', 'const n = try get_n();\n    matrix_n = n;', 1)
            source = source.replace('const len = n / @typeInfo(vec4).vector.len;', 'const len = (n + 3) / 4;')
            source = source.replace('std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(src));', 'std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(src))[0..matrix_n];')
            if identifier == '2':
                source = replace_function(source, 'fn aggregateResults(', '''fn aggregateResults(u: []const vec4, v: []const vec4) f64 {
    const us = std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(u));
    const vs = std.mem.bytesAsSlice(f64, std.mem.sliceAsBytes(v));
    var vbv: f64 = 0;
    var vv: f64 = 0;
    for (0..matrix_n) |i| {
        vbv += us[i] * vs[i];
        vv += vs[i] * vs[i];
    }
    return @sqrt(vbv / vv);
}''')
                source = re.sub(r'    const task_count =.*?defer allocator.free\(tasks\);\n', '', source, flags=re.S)
            else:
                source = replace_function(source, 'fn aggregateResults(', '''fn aggregateResults(first: usize, last: usize, u: []const vec4, v: []const vec4, total_vbv: *f64, total_vv: *f64) void {
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
}''')
            changes.append('Handle arbitrary matrix dimensions by padding output vector lanes while scanning exactly N source values and reducing only N final elements; no matrix remainder is dropped.')
    if benchmark == 'mandelbrot':
        source = source.replace('    var hash: [16]u8 = undefined;\n    md5.hash(pixels.items, &hash, .{});\n    try stdout.print("{}\\n", .{std.fmt.fmtSliceHexLower(&hash)});', '    _ = try stdout.write(pixels.items);')
        changes.append('Emit the complete PBM pixel payload instead of its MD5 checksum.')
        source = source.replace('var terminate = true;\n        var i: u8 = 0;\n        while (i < VEC_SIZE) : (i += 1) {\n            if (absz[i] <= 4.0) {\n                terminate = false;\n                break;\n            }\n        }\n        if (terminate)', 'if (!@reduce(.Or, absz <= @as(Vec, @splat(4.0))))')
        source = source.replace('var i: u8 = 0;\n    while (i < VEC_SIZE) : (i += 1) {', 'inline for (0..VEC_SIZE) |i| {')
        changes.append('Use equivalent portable vector comparisons and compile-time byte-packing indexes supported by Zig 0.17.')
    if benchmark == 'fannkuchredux' and identifier == '2':
        source = source.replace('fn apply_mask(a: Vec, n: u8, comptime mask: anytype) Vec {', 'fn apply_mask(a: Vec, n: u8, comptime mask: anytype) Vec {\n    @setEvalBranchQuota(100000);')
    if benchmark == 'fannkuchredux' and identifier == '2-m':
        source = source.replace('const Vec = @Vector(max_n, u8);','const Vec = @Vector(16, u8);')
        source = source.replace('8, 9, 10, 11 }','8, 9, 10, 11, 12, 13, 14, 15 }')
        source = source.replace('inline while (i < max_n)', 'inline while (i <= max_n)')
        source = source.replace('fn applyMask(a: Vec, n: u8, comptime mask: anytype) Vec {','fn applyMask(a: Vec, n: u8, comptime mask: anytype) Vec {\n    @setEvalBranchQuota(100000);')
        source = source.replace('    const n = try get_n();','    const n = try get_n();\n    std.debug.assert(n >= 1 and n <= max_n);')
        changes.append('Include the declared maximum N in factorial/shuffle dispatch. The upstream exclusive bound silently enumerated only 11! at N=12 in ReleaseFast; preserve disjoint parallel coverage and use a portable 16-byte SIMD vector.')
    if benchmark == 'nbody' and identifier in ['2', '3']:
        source = source.replace('@Vector(3, f64)', '@Vector(4, f64)')
        source = re.sub(r'(\.pos = \.\{)([^{}]+)(\})', lambda m: m[1]+m[2]+', 0 '+m[3], source)
        source = re.sub(r'(vec3\{)([^{}]+)(\})', lambda m: m[1]+m[2]+', 0 '+m[3], source)
        begin = source.index('var solar_bodies =')
        end = source.index('\npub fn main', begin)
        initializer = source[begin:end]
        bodies = re.findall(r'Body\{(.*?)\n    \}', initializer, re.S)
        rows = []
        for body in bodies:
            pos = re.search(r'\.pos = (.*?),\n', body)[1]
            vel = re.search(r'\.vel = (.*?),\n', body)[1]
            mass = re.search(r'\.mass = (.*?),(?:\n|$)', body)[1]
            positions = ['0'] * 3 if '@splat' in pos else pos[pos.index('{')+1:pos.index('}')].split(',')[:3]
            velocities = ['0'] * 3 if '@splat' in vel else [v.strip()+' * year' for v in vel[vel.index('{')+1:vel.index('}')].split(',')[:3]]
            rows.append('    .{'+', '.join([*positions, *velocities, mass])+'},')
        replacement = 'const body_data = [5][7]f64{\n'+'\n'.join(rows)+'\n};\nvar solar_bodies: [5]Body = undefined;\nfn initialize_bodies() void {\n    const seeds: *volatile const [5][7]f64 = &body_data;\n    for (0..5) |i| {\n        const row = seeds.*[i];\n        solar_bodies[i].pos = .{row[0], row[1], row[2], 0};\n        solar_bodies[i].vel = .{row[3], row[4], row[5], 0};\n        solar_bodies[i].mass = row[6];\n    }\n}\n'
        source = source[:begin]+replacement+source[end:]
        source = source.replace('    const steps = try ', '    initialize_bodies();\n    const steps = try ', 1)
        changes.append('Use zero-padded four-lane position/velocity vectors to avoid the Zig 0.17 native global-initializer layout error for three-lane vectors; all three physical coordinates and pair interactions are retained.')
        changes.append('Initialize vector-containing body records at runtime from a scalar seed table, avoiding the compiler global record-layout defect. This startup work is included in timing and LOC.')
    if benchmark == 'knucleotide':
        if identifier == '1-m':
            source = source.replace('const gpa = std.heap.c_allocator;', 'const global_allocator = std.heap.c_allocator;\nconst gpa = global_allocator;')
        source = replace_function(source, 'pub fn readInput()', ZIG_INPUT)
        source = replace_function(source, 'pub inline fn encodeByte(', '''pub inline fn encodeByte(c: u8) u2 {
        return switch (c) { 'A' => 0, 'C' => 1, 'G' => 2, 'T' => 3, else => unreachable };
    }''')
        source = source.replace('@as(f32,', '@as(f64,')
        source = source.replace('std.BoundedArray(CountCode, code_limit)', 'std.array_list.Managed(CountCode)')
        if identifier == '1-m':
            source = source.replace('var cc: std.array_list.Managed(CountCode) = .{};', 'var cc = std.array_list.Managed(CountCode).init(gpa);\n    defer cc.deinit();\n    try cc.ensureTotalCapacity(code_limit);')
            source = source.replace('cc.slice()', 'cc.items')
        changes.append('Read the complete THREE record from stdin, uppercase DNA, retain all seven full growable-map scans, and use lexical tie ordering and binary64 percentages.')
    # New compiler diagnostic: allocated slice bindings are immutable even when elements mutate.
    for name in ['perm', 'perm1', 'count', 'node']:
        source = re.sub(r'\bvar ' + name + r'(?= = (?:try allocator\.alloc|Self\.init))', 'const ' + name, source)
    return source, changes

def odin(source, benchmark, identifier):
    changes = []
    if benchmark == 'spectralnorm':
        source = source.replace('x := make([]f64, n)', 'x := make([]f64, n)\n\tdefer delete(x)')
        changes.append('Free the standard temporary vector after every complete A-transpose-A product.')
    if benchmark == 'mandelbrot':
        source = source.replace('import "core:crypto/legacy/md5"', 'import "core:os"').replace('import "core:strings"\n', '')
        start = source.index('\thash := md5hash(pixels)')
        end = source.index('\nmbrot8 ::')
        source = source[:start] + '\t_, err := os.write(os.stdout, pixels)\n\tassert(err == nil)\n}\n' + source[end:]
        changes.append('Emit the complete PBM image instead of the MD5 checksum; pixel computation is retained.')
    return source, changes

def main():
    bootstrap = json.loads((D / 'bootstrap.json').read_text())
    rows = []
    for record in bootstrap['sources']:
        path = Path(record['path'])
        if path.suffix not in ('.zig', '.odin'): continue
        task = path.parent.name.replace('-', '')
        if task == 'spectralnorm': task = 'spectralnorm'
        language = path.suffix[1:]
        identifier = path.stem
        if language == 'odin' and task == 'knucleotide':
            continue  # Its fixed-table implementation is preserved; a standard growable-map port follows.
        if language == 'zig' and task == 'fannkuchredux' and identifier == '3-i':
            continue  # Its x86 byte shuffle is preserved; ARM table-lookup port follows.
        source, changes = (zig if language == 'zig' else odin)(path.read_text(), task, identifier)
        target = D / 'sources' / f'{task}-{language}-{identifier}-faithful.{language}'
        target.write_text(source)
        rows.append(dict(key=task + ':' + target.stem, id=target.stem, benchmark=task, language=language,
            source=str(target), source_sha256=sha(target), upstream_source=str(path), upstream_sha256=record['sha256'],
            source_url=record['source_url'], local_adaptation=bool(changes), adaptations=changes))
    (D / 'published.json').write_text(json.dumps(dict(programs=rows), indent=2) + '\n')
    print('PREPARED', len(rows), flush=True)

if __name__ == '__main__': main()
