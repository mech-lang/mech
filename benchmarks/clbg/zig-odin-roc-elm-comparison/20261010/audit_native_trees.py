"""Untimed allocation-count instrumentation of the four measured arena sources."""
import concurrent.futures
import json
import re
import subprocess
from pathlib import Path
from bootstrap import D, sha
from build import build


def expected(n):
    maximum = max(6, n)
    total = (2 ** (maximum + 2) - 1) + (2 ** (maximum + 1) - 1)
    return total + sum(2 ** (maximum - depth + 4) * (2 ** (depth + 1) - 1)
                       for depth in range(4, maximum + 1, 2))


def instrument(row):
    text = Path(row['source']).read_text()
    if row['language'] == 'zig':
        text = text.replace('const heap = std.heap.c_allocator;',
            'const heap = std.heap.c_allocator;\nvar allocated = std.atomic.Value(usize).init(0);')
        text = text.replace('const node = try allocator.create(Node);',
            'const node = try allocator.create(Node);\n    _ = allocated.fetchAdd(1, .monotonic);')
        text = text.replace('node.right = try build(allocator, depth - 1);',
            'node.right = try build(allocator, depth - 1);\n        if (node.left == node.right) return error.SharedChildren;')
        text = text.rstrip()[:-1] + '    std.debug.print("AUDIT {d} {d}\\n", .{ @sizeOf(Node), allocated.load(.monotonic) });\n}\n'
    else:
        text = text.replace('import "core:fmt"', 'import "core:fmt"\nimport "core:sync"\nallocated: int')
        text = text.replace('node := new(Node, allocator)',
            'node := new(Node, allocator)\n    _ = sync.atomic_add(&allocated, 1)')
        text = text.replace('node.right = build(depth-1, allocator)',
            'node.right = build(depth-1, allocator)\n        if node.left == node.right { panic("Shared children") }')
        text = text.rstrip()[:-1] + '    fmt.eprintf("AUDIT %d %d\\n", size_of(Node), sync.atomic_load(&allocated))\n}\n'
    path = D / 'evidence' / ('audit-' + row['id'] + '.' + row['language'])
    path.write_text(text)
    copy = dict(row, id='audit-'+row['id'], key='audit:'+row['key'], source=str(path), source_sha256=sha(path))
    return copy


def main():
    rows = json.loads((D/'results/final-extra-builds.json').read_text())['programs']
    rows = [row for row in rows if row['language'] in ('zig','odin')]
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        instrumented = list(pool.map(build, [instrument(row) for row in rows]))
    assert all(row['build_status']=='passed' for row in instrumented)
    results = []
    for original, instrumented_row in zip(rows, instrumented):
        cases = []
        for n in (9,10):
            baseline = subprocess.run([original['binary'],str(n)],check=True,capture_output=True)
            run = subprocess.run([instrumented_row['binary'],str(n)],check=True,capture_output=True)
            assert run.stdout == baseline.stdout
            match = re.fullmatch(rb'AUDIT (\d+) (\d+)\n',run.stderr)
            assert match, run.stderr
            size,count = map(int,match.groups())
            assert size == 16 and count == expected(n), (original['id'],n,size,count,expected(n))
            cases.append(dict(n=n,node_bytes=size,allocation_requests=count,expected_allocation_requests=expected(n),complete_output_unchanged=True,distinct_children=True))
        results.append(dict(key=original['key'],status='passed',source_sha256=original['source_sha256'],binary_sha256=original['binary_sha256'],
            instrumentation_source=instrumented_row['source'],instrumentation_source_sha256=instrumented_row['source_sha256'],
            instrumentation_binary=instrumented_row['binary'],instrumentation_binary_sha256=instrumented_row['binary_sha256'],cases=cases))
        print('ALLOCATION PASS',original['id'],cases[-1]['allocation_requests'],flush=True)
    (D/'results/native-tree-allocation-final.json').write_text(json.dumps(dict(status='passed',programs=results,
        reason='Untimed derivative builds count every explicit create/new request using a shared atomic counter, assert distinct interior children, verify the unchanged complete output, and retain equal-size 16-byte leaves/interiors. Measurements use the uninstrumented pinned binaries. General arenas reset only after construction and pointer traversal, and long-lived storage uses a separate arena.'),indent=2)+'\n')


if __name__=='__main__': main()
