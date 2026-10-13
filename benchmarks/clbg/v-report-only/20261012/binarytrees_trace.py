"""Draft unscored V allocation/retention trace; no builds or native launches.

The creator, Node fields, check arithmetic, and program stdout stay byte exact.
Each root uses a fresh map of scalar addresses. No extra Node is constructed,
and temporary roots are not retained by the observer. Root owns qualification.
"""
from pathlib import Path
import re

from source_trace import _owned_mutex

SIZES = (6, 7, 9)
VARIANTS = {'compact', 'optimized'}
TRACE_ENV = 'MECH_V_TREE_TRACE'


OBSERVER = r'''
// Unscored isolated observer only; absent from every measured source.
fn audit_tree_walk(tree &Node, level int, mut seen map[u64]bool, mut trace os.File) int {
    if isnil(tree) {
        panic('Missing actual node')
    }
    address := unsafe { u64(usize(tree)) }
    if address == 0 || address in seen {
        panic('Repeated actual node address within one tree')
    }
    seen[address] = true
    left := unsafe { u64(usize(tree.left)) }
    right := unsafe { u64(usize(tree.right)) }
    trace.writeln('NODE|${address}|${left}|${right}|${level}') or { panic(err) }
    if level == 0 {
        if !isnil(tree.left) || !isnil(tree.right) {
            panic('Actual leaf has a child')
        }
        return 1
    }
    if level < 0 || isnil(tree.left) || isnil(tree.right) {
        panic('Actual internal node lacks both children')
    }
    return 1 + audit_tree_walk(tree.left, level - 1, mut seen, mut trace) +
        audit_tree_walk(tree.right, level - 1, mut seen, mut trace)
}
fn audit_tree(tree &Node, kind string, depth int, index int, cumulative int) {
    directory := os.getenv('MECH_V_TREE_TRACE')
    if directory.len == 0 || !os.is_dir(directory) {
        panic('An existing isolated tree trace directory is required')
    }
    path := os.join_path(directory, '${kind}-${depth}-${index}.trace')
    if os.exists(path) {
        panic('Duplicate root trace; previous evidence must not be overwritten')
    }
    mut trace := os.create(path) or { panic(err) }
    defer {
        trace.close()
    }
    address := unsafe { u64(usize(tree)) }
    trace.writeln('TRACE_TREE_V1|${kind}|${depth}|${index}|${address}|${cumulative}') or { panic(err) }
    mut seen := map[u64]bool{}
    count := audit_tree_walk(tree, depth, mut seen, mut trace)
    trace.writeln('END_TREE_V1|${count}|${seen.len}') or { panic(err) }
}
'''


def _function(source, name):
    match = re.search(r'^fn '+re.escape(name)+r'\([^\n]*\)[^\n]*\{\n', source, re.MULTILINE)
    assert match, ('Missing exact function anchor', name)
    # The reviewed functions use a closing brace at column zero; nested blocks
    # are indented. Avoid a general rewriting/parsing pass over production code.
    closing = source.find('\n}\n', match.end())
    assert closing >= 0, ('Missing exact function close', name)
    return source[match.start():closing+3]


def instrument(source, variant):
    """Return a new observer copy; never write or mutate a measured source."""
    assert variant in VARIANTS
    assert 'audit_tree' not in source and TRACE_ENV not in source
    original_node = re.search(r'^struct Node \{\n.*?^\}', source, re.MULTILINE | re.DOTALL)
    assert original_node, 'Missing original Node declaration'
    original_create = _function(source, 'create')
    original_check = _function(source, 'check')
    stretch = _function(source, 'stretch')
    anchor = '    tree := create(depth)\n'
    assert stretch.count(anchor) == 1
    observed_stretch = stretch.replace(anchor, anchor+"    audit_tree(tree, 'stretch', depth, 0, 0)\n")
    result = source.replace(stretch, observed_stretch, 1)
    loop = '    for _ in 0 .. count {\n' if variant == 'optimized' else '        for _ in 0 .. count {\n'
    assert result.count(loop) == 1, 'Expected exactly one real temporary-tree loop'
    result = result.replace(loop, loop.replace('for _', 'for audit_index'), 1)
    update = '        sum += check(tree)\n' if variant == 'optimized' else '            sum += check(tree)\n'
    assert result.count(update) == 1
    indentation = update[:len(update)-len(update.lstrip())]
    result = result.replace(update, update+indentation+"audit_tree(tree, 'temporary', depth, audit_index, sum)\n", 1)
    long_start = '    long_lived := create(maximum)\n'
    assert result.count(long_start) == 1
    result = result.replace(long_start, long_start+"    audit_tree(long_lived, 'long-before', maximum, 0, 0)\n", 1)
    long_print = "    println('long lived tree of depth ${maximum}\\t check: ${check(long_lived)}')\n"
    assert result.count(long_print) == 1
    result = result.replace(long_print, "    audit_tree(long_lived, 'long-after', maximum, 0, 0)\n"+long_print, 1)
    result += OBSERVER
    assert _function(result, 'create') == original_create
    assert _function(result, 'check') == original_check
    assert re.search(r'^struct Node \{\n.*?^\}', result, re.MULTILINE | re.DOTALL)[0] == original_node[0]
    assert result.count('&Node{') == source.count('&Node{'), 'Observer must not construct any Node'
    assert [line.strip() for line in source.splitlines() if line.strip().startswith('println(')] == [
        line.strip() for line in result.splitlines() if line.strip().startswith('println(')]
    return result


def _integer(value):
    assert value and value.isascii() and value.isdecimal(), ('Invalid unsigned decimal field', value)
    return int(value)


def _read_tree(path):
    lines = Path(path).read_text().splitlines()
    assert len(lines) >= 3 and lines[0].startswith('TRACE_TREE_V1|')
    fields = lines[0].split('|')
    assert len(fields) == 6
    _, kind, depth, index, root, cumulative = fields
    depth, index, root, cumulative = map(_integer, (depth, index, root, cumulative))
    assert kind in {'stretch', 'temporary', 'long-before', 'long-after'}
    assert root > 0
    nodes = {}
    for line in lines[1:-1]:
        fields = line.split('|')
        assert len(fields) == 5 and fields[0] == 'NODE'
        address, left, right, level = map(_integer, fields[1:])
        assert address > 0 and address not in nodes, 'Repeated real address in one root'
        nodes[address] = (left, right, level)
    ending = lines[-1].split('|')
    assert len(ending) == 3 and ending[0] == 'END_TREE_V1'
    actual_count, map_count = map(_integer, ending[1:])
    expected_count = (1 << (depth + 1))-1
    assert actual_count == map_count == len(nodes) == expected_count
    visited = set()

    def walk(address, level):
        assert address in nodes and address not in visited, 'Missing, shared or cyclic actual node'
        visited.add(address)
        left, right, actual_level = nodes[address]
        assert actual_level == level
        if level == 0:
            assert left == right == 0, 'Actual leaf must have two nil children'
            return 1
        assert left > 0 and right > 0
        return 1 + walk(left, level-1) + walk(right, level-1)

    assert walk(root, depth) == expected_count
    assert visited == set(nodes), 'Trace contains unreachable unused nodes'
    return dict(kind=kind, depth=depth, index=index, root=root, cumulative=cumulative,
                nodes=nodes, count=expected_count)


def _expected_stdout(maximum):
    lines = [f'stretch tree of depth {maximum+1}\t check: {(1 << (maximum+2))-1}']
    for depth in range(4, maximum+1, 2):
        count = 1 << (maximum-depth+4)
        lines.append(f'{count}\t trees of depth {depth}\t check: {count*((1 << (depth+1))-1)}')
    lines.append(f'long lived tree of depth {maximum}\t check: {(1 << (maximum+1))-1}')
    return ('\n'.join(lines)+'\n').encode()


def reconcile(directory, stdout, n, variant, lock_fd):
    """Reconcile all actual root graphs and full stdout under the held mutex."""
    _owned_mutex(lock_fd)
    assert variant in VARIANTS and n in SIZES, 'Only the approved bounded tree sizes'
    maximum = max(6, n)
    expected = {('stretch', maximum+1, 0), ('long-before', maximum, 0), ('long-after', maximum, 0)}
    for depth in range(4, maximum+1, 2):
        count = 1 << (maximum-depth+4)
        expected.update(('temporary', depth, index) for index in range(count))
    paths = sorted(Path(directory).iterdir())
    assert all(p.is_file() and p.suffix == '.trace' for p in paths), 'Unexpected observer artifact'
    observed = {}
    for path in paths:
        row = _read_tree(path)
        key = row['kind'], row['depth'], row['index']
        assert key in expected and key not in observed, ('Unexpected/duplicate actual root', key)
        assert path.name == f'{key[0]}-{key[1]}-{key[2]}.trace'
        wanted_sum = (row['index']+1)*row['count'] if row['kind'] == 'temporary' else 0
        assert row['cumulative'] == wanted_sum, 'Actual repeated check sum disagrees with independent count'
        observed[key] = row
    assert set(observed) == expected, 'Missing prescribed roots/iterations or retention snapshots'
    before = observed['long-before', maximum, 0]
    after = observed['long-after', maximum, 0]
    assert before['root'] == after['root'] and before['nodes'] == after['nodes'], 'Retained root/forest changed'
    retained = set(before['nodes'])
    for key, row in observed.items():
        if key[0] == 'temporary':
            assert not retained.intersection(row['nodes']), 'Temporary root aliases retained actual objects'
    assert stdout == _expected_stdout(maximum), 'Complete original binarytrees stdout mismatch'
    return dict(status='passed', benchmark='binarytrees', variant=variant, n=n,
        actual_roots=len(observed), temporary_roots=sum(k[0]=='temporary' for k in observed),
        actual_nodes_traversed=sum(row['count'] for row in observed.values()),
        distinct_nodes_and_leaves_per_root=True, retained_root_and_all_edges_identical=True,
        temporary_graphs_disjoint_from_live_retained_graph=True, complete_stdout_match=True,
        creator_check_and_node_declaration_unchanged=True,
        temporary_address_reuse_between_different_roots_allowed=True,
        observer_retains_no_extra_node_objects=True,
        full_workload=False, never_chart_eligible=True, scored_samples=0)
