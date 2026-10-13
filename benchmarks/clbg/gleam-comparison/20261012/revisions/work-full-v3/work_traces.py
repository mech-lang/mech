"""Unscored Gleam graph/map/Pi observers; original scored BEAM proof remains separate."""
from pathlib import Path
from collections import Counter
import re


def replace_once(source, anchor, replacement):
    assert source.count(anchor) == 1, ('Observer anchor is not unique', anchor)
    return source.replace(anchor, replacement)


TRACE_GLEAM = '''pub type Handle
@external(erlang, "clbg_trace_ffi", "begin")
pub fn begin(suffix: String) -> Nil
@external(erlang, "clbg_trace_ffi", "line")
pub fn line(suffix: String, value: String) -> Nil
@external(erlang, "clbg_trace_ffi", "identity")
pub fn identity(value: a) -> Int
@external(erlang, "clbg_trace_ffi", "pid")
pub fn pid() -> Int
@external(erlang, "erts_debug", "flat_size")
pub fn words(value: a) -> Int
'''
TRACE_ERLANG = '''-module(clbg_trace_ffi).
-export(['begin'/1, line/2, identity/1, pid/0]).
path(Suffix) ->
    Base = os:getenv("CLBG_TRACE"),
    true = is_list(Base),
    Base ++ binary_to_list(Suffix).
'begin'(Suffix) ->
    ok = file:write_file(path(Suffix), <<>>, [exclusive]),
    nil.
line(Suffix, Value) ->
    ok = file:write_file(path(Suffix), [Value, <<10>>], [append]),
    nil.
identity(Value) -> binary:decode_unsigned(term_to_binary(Value)).
pid() -> identity(self()).
'''
TREE_HELPER = '''fn audit_identity(tree: Tree) -> Int {
  case tree { Leaf(identity) -> trace.identity(identity) Node(identity, _, _) -> trace.identity(identity) }
}

fn audit_nodes(tree: Tree, suffix: String) -> Nil {
  case tree {
    Leaf(identity) -> trace.line(suffix, "NODE " <> io.integer(trace.identity(identity)) <> " 0 0")
    Node(identity, left, right) -> {
      trace.line(suffix, "NODE " <> io.integer(trace.identity(identity)) <> " " <> io.integer(audit_identity(left)) <> " " <> io.integer(audit_identity(right)))
      audit_nodes(left, suffix)
      audit_nodes(right, suffix)
    }
  }
}

fn audit_root(tree: Tree, depth: Int, tag: String, ordinal: Int, suffix: String) -> Nil {
  trace.line(suffix, "ROOT " <> tag <> " " <> io.integer(depth) <> " " <> io.integer(ordinal) <> " " <> io.integer(audit_identity(tree)))
  audit_nodes(tree, suffix)
  trace.line(suffix, "ENDROOT")
}

'''


def tree_source(source, optimized):
    source = 'import clbg_trace as trace\n' + source
    source = replace_once(source, 'fn make(depth: Int) -> Tree {', TREE_HELPER + 'fn make(depth: Int) -> Tree {')
    source = replace_once(source, '  let requested = io.argument()',
        '  trace.begin("-main")\n  trace.line("-main", "MAIN " <> io.integer(trace.pid()))\n  let requested = io.argument()')
    stretch = '  io.text("stretch tree of depth " <> io.integer(maximum + 1) <> "\\t check: " <> io.integer(check(make(maximum + 1))) <> "\\n")'
    source = replace_once(source, stretch, '''  let stretch = make(maximum + 1)
  audit_root(stretch, maximum + 1, "stretch", 0, "-main")
  io.text("stretch tree of depth " <> io.integer(maximum + 1) <> "\\t check: " <> io.integer(check(stretch)) <> "\\n")''')
    source = replace_once(source, '  let long_lived = make(maximum)',
        '  let long_lived = make(maximum)\n  audit_root(long_lived, maximum, "long-open", 0, "-main")')
    source = replace_once(source, '  depths(4, maximum)',
        '  depths(4, maximum)\n  audit_root(long_lived, maximum, "long-close", 0, "-main")')
    if not optimized:
        source = replace_once(source, '    False -> sum(count - 1, depth, total + check(make(depth)))', '''    False -> {
      let tree = make(depth)
      audit_root(tree, depth, "iteration", count - 1, "-main")
      sum(count - 1, depth, total + check(tree))
    }''')
    else:
        source = replace_once(source, 'fn sum(first: Int, last: Int, depth: Int, total: Int) -> Int {',
            'fn sum(first: Int, last: Int, depth: Int, total: Int, suffix: String) -> Int {')
        source = replace_once(source, '    False -> sum(first + 1, last, depth, total + check(make(depth)))', '''    False -> {
      let tree = make(depth)
      audit_root(tree, depth, "iteration", first, suffix)
      sum(first + 1, last, depth, total + check(tree), suffix)
    }''')
        source = replace_once(source, '-> List(process.Task(Int)) {', '-> List(#(process.Task(Int), Int, Int)) {')
        source = replace_once(source, '      let task = process.start(fn() { sum(first, last, depth, 0) })', '''      let suffix = "-" <> io.integer(depth) <> "-" <> io.integer(first)
      let task = process.start(fn() {
        trace.begin(suffix)
        trace.line(suffix, "RANGE " <> io.integer(depth) <> " " <> io.integer(first) <> " " <> io.integer(last) <> " " <> io.integer(trace.pid()))
        sum(first, last, depth, 0, suffix)
      })''')
        source = replace_once(source, '[task, ..workers(index + 1, count, iterations, depth)]',
            '[#(task, first, last), ..workers(index + 1, count, iterations, depth)]')
        source = replace_once(source, '      let totals = list.map(workers(0, count, iterations, depth), process.join)', '''      let totals = list.map(workers(0, count, iterations, depth), fn(job) {
        let total = process.join(job.0)
        trace.line("-main", "JOIN " <> io.integer(depth) <> " " <> io.integer(job.1) <> " " <> io.integer(job.2))
        total
      })''')
    return source


def pi_source(source):
    source = 'import clbg_trace as trace\n' + source
    if 'True -> list.reverse(lines)' in source:
        source = replace_once(source, '    True -> list.reverse(lines)',
            '    True -> { trace.line("", "END " <> io.integer(n)) list.reverse(lines) }')
    else:
        source = replace_once(source, '    True -> Nil',
            '    True -> { trace.line("", "END " <> io.integer(n)) Nil }')
    source = replace_once(source, '      let numerator3 = 3 * q + r',
        '      trace.line("", "STATE " <> io.integer(q) <> " " <> io.integer(r) <> " " <> io.integer(t) <> " " <> io.integer(k) <> " " <> io.integer(printed))\n      let numerator3 = 3 * q + r')
    source = replace_once(source, '      let digit4 = numerator4 / t',
        '      let digit4 = numerator4 / t\n      trace.line("", "DIV " <> io.integer(numerator3) <> " " <> io.integer(digit3) <> " " <> io.integer(numerator4) <> " " <> io.integer(digit4))')
    source = replace_once(source, 'pub fn main() -> Nil {', 'pub fn main() -> Nil {\n  trace.begin("")')
    return source


MAP_HELPER = '''fn audit_owner(width: Int) -> Nil {
  let suffix = "-" <> io.integer(width)
  trace.begin(suffix)
  trace.line(suffix, "OWNER " <> io.integer(width) <> " " <> io.integer(trace.pid()))
}

fn audit_map(counts: dict.Dict(Int, Int), width: Int) -> Nil {
  let suffix = "-" <> io.integer(width)
  trace.line(suffix, "MAP " <> io.integer(width) <> " " <> io.integer(dict.size(counts)) <> " " <> io.integer(trace.words(counts)) <> " " <> io.integer(trace.pid()))
  list.each(dict.to_list(counts), fn(item) { trace.line(suffix, "ITEM " <> io.integer(item.0) <> " " <> io.integer(item.1)) })
}

'''


def map_source(source, optimized):
    source = 'import clbg_trace as trace\n' + source
    source = replace_once(source, 'fn end_of_line(data: BitArray, offset: Int) -> Int {', MAP_HELPER + 'fn end_of_line(data: BitArray, offset: Int) -> Int {')
    if optimized:
        source = replace_once(source, '    process.start(fn() { histogram(sequence, width, 0, 0, power_four(width, 1), dict.new()) })',
            '    process.start(fn() { audit_owner(width) histogram(sequence, width, 0, 0, power_four(width, 1), dict.new()) })')
    else:
        source = replace_once(source, 'fn(width) { histogram(sequence, width, 0, dict.new()) }',
            'fn(width) { audit_owner(width) histogram(sequence, width, 0, dict.new()) }')
    source = replace_once(source, '  let assert [one, two, three, four, six, twelve, eighteen] = counts',
        '  list.each(list.zip(counts, [1, 2, 3, 4, 6, 12, 18]), fn(item) { audit_map(item.0, item.1) })\n  let assert [one, two, three, four, six, twelve, eighteen] = counts')
    return source


def map_trace(directory, data, optimized, references, require_growth=False):
    sequence, expected = references.nucleotide_maps(data)
    owners, readers, rows = [], [], []
    for width, wanted in zip(references.WIDTHS, expected):
        lines = (Path(directory) / ('trace-' + str(width))).read_text().splitlines()
        owner, header = lines[0].split(), lines[1].split()
        assert owner[0] == 'OWNER' and int(owner[1]) == width and int(owner[2]) > 0
        assert header[0] == 'MAP' and int(header[1]) == width and len(header) == 5
        length, words, reader = map(int, header[2:])
        assert words > 0 and reader > 0
        owners.append(int(owner[2])); readers.append(reader)
        actual = {}
        for line in lines[2:]:
            tag, key, count = line.split()
            assert tag == 'ITEM'
            key, count = int(key), int(count)
            assert 0 <= key < (1 << (2 * width)) and count > 0
            word = ''.join('ACGT'[(key >> (2 * shift)) & 3] for shift in range(width - 1, -1, -1))
            assert word not in actual
            actual[word] = count
        assert actual == dict(wanted) and len(actual) == length
        windows = max(0, len(sequence) - width + 1)
        assert sum(actual.values()) == windows
        if require_growth and width >= 3:
            assert length > 32 and words > 32, 'Actual immutable map must grow beyond flat-map threshold'
        rows.append(dict(width=width, entries=length, windows=windows, actual_heap_words=words))
    assert len(set(readers)) == 1
    if optimized:
        assert len(set(owners)) == 7 and all(owner != readers[0] for owner in owners)
    else:
        assert owners == readers
    return dict(status='passed', actual_seven_complete_maps=True, complete_map_counts_and_window_totals_match=True,
        heap_owned_tables_after_actual_joins=True, source_map_growth=require_growth, maps=rows,
        ownership='Ordinary immutable BEAM process-owned map values, copied/delivered to main after actual joins; no pointer/allocator-address claim')


def beam_functions(text):
    starts = list(re.finditer(r'\{function,\s*([a-z_][a-z_0-9]*),\s*(\d+),\s*(\d+),', text))
    assert starts, 'No complete actual scored BEAM disassembly functions'
    functions = {}
    for i, match in enumerate(starts):
        identity = (match[1], int(match[2]))
        assert identity not in functions
        end = starts[i + 1].start() if i + 1 < len(starts) else len(text)
        functions[identity] = (int(match[3]), text[match.end():end])
    return functions


def allocation_beam(text, reference_text):
    functions = beam_functions(text)
    label, creator = functions[('make', 1)]
    fresh = r'\{extfunc,\s*(?:clbg_reference,\s*new|erlang,\s*make_ref),\s*0\}'
    assert re.search(fresh, creator), 'Scored creator must obtain fresh real identity; pinned Gleam may inline its external call'
    assert re.search(r'\{extfunc,\s*erlang,\s*make_ref,\s*0\}', reference_text), 'Scored reference wrapper must call actual make_ref'
    leaf = re.search(r'put_tuple2,.*?\{list,\s*\[\{atom,\s*leaf\},\s*\{[xy],\d+\}', creator, re.S)
    node = re.search(r'put_tuple2,.*?\{list,\s*\[\{atom,\s*node\},\s*\{[xy],\d+\},\s*\{[xy],\d+\},\s*\{[xy],\d+\}', creator, re.S)
    assert leaf and node, 'Original scored leaf/interior paths require dynamic tuples with live identity/children'
    child = list(re.finditer(r'\{call(?:_last|_only)?,\s*1,\s*\{f,\s*' + str(label) + r'\}', creator))
    assert len(child) >= 2, 'Original scored creator must actually construct both children'
    main = functions[('main', 0)][1]
    make_calls = list(re.finditer(r'\{call(?:_last|_only)?,\s*1,\s*\{f,\s*' + str(label) + r'\}', main))
    depth_label, _ = functions[('depths', 2)]
    check_label, _ = functions[('check', 1)]
    churn = re.search(r'\{call(?:_last|_only)?,\s*2,\s*\{f,\s*' + str(depth_label) + r'\}', main)
    checks = list(re.finditer(r'\{call(?:_last|_only)?,\s*1,\s*\{f,\s*' + str(check_label) + r'\}', main))
    assert len(make_calls) == 2 and churn and checks
    assert make_calls[1].end() < churn.start() < checks[-1].start()
    before = main[make_calls[1].end():churn.start()]
    saved = re.search(r'\{move,\s*\{x,\s*0\},\s*(\{y,\s*\d+\})\}', before)
    assert saved, 'Original long-lived root must survive churn in a live stack slot'
    after = main[churn.end():checks[-1].start()]
    restored = re.search(r'\{move,\s*' + re.escape(saved[1]) + r',\s*\{x,\s*0\}\}', after)
    assert restored, 'Post-churn check must use the same retained stack root'
    live_interval = before[saved.end():] + main[churn.start():churn.end()] + after[:restored.start()]
    slot = re.escape(saved[1])
    assert not re.search(r'\{move,\s*\{(?:x|y|atom|integer),[^{}]+\},\s*' + slot + r'\}', live_interval), 'Retained long-root stack slot overwritten before its actual later check'
    assert not re.search(r'\{init_yregs,.*?' + slot, live_interval, re.S), 'Retained long-root stack slot reset during churn'
    return dict(status='passed', fresh_reference_identity=True, dynamic_leaf_and_interior_tuples=True,
        both_actual_child_calls=True, retained_scored_long_lived_root=True,
        proof='Original scored BEAM tuple/reference operations and retained stack-slot use; actual native ARM64 engine bound by qualified runtime receipts')

def tree_trace(directory, n, optimized):
    root_depth = max(6, n)
    expected = {(d, i) for d in range(4, root_depth + 1, 2)
                for i in range(1 << (root_depth - d + 4))}
    found = set()
    graphs = {}
    ranges = []
    joins = []
    main_tid = None
    total_nodes = 0
    live_churn_node_sets = []
    main_order = []
    files = sorted(Path(directory).glob('trace-*'))
    assert files
    for path in files:
        lines = path.read_text().splitlines()
        position = 0
        file_ranges = []
        file_iterations = []
        is_main = path.name == 'trace-main'
        while position < len(lines):
            fields = lines[position].split()
            position += 1
            if fields[0] == 'MAIN':
                assert is_main and main_tid is None
                main_tid = int(fields[1])
                continue
            if fields[0] == 'RANGE':
                interval = tuple(map(int, fields[1:]))
                assert not is_main and not file_ranges and position == 1
                ranges.append(interval)
                file_ranges.append(interval)
                continue
            if fields[0] == 'JOIN':
                assert is_main
                joins.append(tuple(map(int, fields[1:])))
                main_order.append('JOIN')
                continue
            assert fields[0] == 'ROOT' and len(fields) == 5
            tag, depth, ordinal, root = fields[1], *map(int, fields[2:])
            nodes = {}
            while lines[position] != 'ENDROOT':
                item = lines[position].split()
                position += 1
                assert item[0] == 'NODE' and len(item) == 4
                address, left, right = map(int, item[1:])
                assert address and address not in nodes
                nodes[address] = (left, right)
            position += 1
            visited = set()
            def visit(address, remaining):
                assert address in nodes and address not in visited
                visited.add(address)
                left, right = nodes[address]
                if remaining == 0:
                    assert left == right == 0, 'Actual leaf must exist and have no children'
                else:
                    assert left and right and left != right
                    visit(left, remaining - 1)
                    visit(right, remaining - 1)
            visit(root, depth)
            assert visited == set(nodes) and len(nodes) == (1 << (depth + 1)) - 1
            if tag == 'iteration':
                assert (depth, ordinal) in expected and (depth, ordinal) not in found
                found.add((depth, ordinal))
                file_iterations.append((depth, ordinal))
                live_churn_node_sets.append(set(nodes))
                if is_main:
                    main_order.append('iteration')
                total_nodes += len(nodes)
            else:
                assert is_main
                main_order.append(tag)
                assert tag in ('stretch', 'long-open', 'long-close') and tag not in graphs
                assert ordinal == 0 and depth == (root_depth + 1 if tag == 'stretch' else root_depth)
                graphs[tag] = (root, nodes)
                if tag != 'long-close':
                    total_nodes += len(nodes)
        if optimized and not is_main:
            assert len(file_ranges) == 1
            depth, first, last, tid = file_ranges[0]
            assert file_iterations == [(depth, i) for i in range(first, last)], 'Iteration labels must correlate with their actual worker interval'
        elif optimized:
            assert not file_ranges and not file_iterations
        else:
            assert is_main and not file_ranges
    assert main_order[:2] == ['stretch', 'long-open'] and main_order[-1:] == ['long-close']
    assert all(kind == ('JOIN' if optimized else 'iteration') for kind in main_order[2:-1])
    assert main_tid and found == expected and set(graphs) == {'stretch', 'long-open', 'long-close'}
    assert graphs['long-open'] == graphs['long-close'], 'Long-lived actual graph changed during churn'
    long_nodes = set(graphs['long-open'][1])
    assert all(not (nodes & long_nodes) for nodes in live_churn_node_sets), 'Concurrently live churn graph overlaps the retained long-lived graph'
    # The stretch graph was released before long-open; allocator address reuse
    # from that earlier graph is valid and is deliberately not forbidden.
    if optimized:
        expected_ranges = []
        for depth in range(4, root_depth + 1, 2):
            iterations = 1 << (root_depth - depth + 4)
            count = min(8, iterations)
            expected_ranges.extend((depth, iterations * i // count, iterations * (i + 1) // count)
                                   for i in range(count))
        assert sorted(row[:3] for row in ranges) == sorted(joins) == sorted(expected_ranges)
        assert all(tid and tid != main_tid for _, _, _, tid in ranges)
        for depth in range(4, root_depth + 1, 2):
            assert len({tid for d, _, _, tid in ranges if d == depth}) == 8
    else:
        assert ranges == joins == []
    return dict(status='passed', actual_distinct_nodes_and_leaf_objects=True,
                actual_stretch_and_all_iteration_roots=True, unchanged_long_lived_graph_after_churn=True,
                joined_actual_worker_intervals=True, actual_interval_labels_correlated=True,
                concurrent_churn_disjoint_from_retained_graph=True, observed_allocated_nodes=total_nodes,
                actual_iteration_roots=len(found), files=[str(p) for p in files])



def pi_trace(directory, n):
    lines = (Path(directory) / 'trace').read_text().splitlines()
    assert lines[-1:] == ['END ' + str(n)]
    assert (len(lines) - 1) % 2 == 0
    q, r, t, k, printed = 1, 0, 1, 1, 0
    accepted = rejected = 0
    for i in range(0, len(lines) - 1, 2):
        state = lines[i].split()
        divisions = lines[i + 1].split()
        assert state[0] == 'STATE' and list(map(int, state[1:])) == [q, r, t, k, printed]
        a, b = 3 * q + r, 4 * q + r
        assert a >= 0 and b >= 0 and t > 0
        da, db = a // t, b // t
        assert divisions[0] == 'DIV' and list(map(int, divisions[1:])) == [a, da, b, db]
        if da == db:
            assert 0 <= da <= 9
            printed += 1
            q, r = q * 10, (r - da * t) * 10
            accepted += 1
        else:
            factor = 2 * k + 1
            q, r, t, k = q * k, (2 * q + r) * factor, t * factor, k + 1
            rejected += 1
    assert printed == accepted == n
    return dict(status='passed', actual_both_division_operands_and_quotients=True,
                independent_recurrence=True, accepted_candidates=accepted, rejected_candidates=rejected)

