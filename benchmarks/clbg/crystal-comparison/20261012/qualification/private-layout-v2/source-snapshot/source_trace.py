"""Unscored Crystal observation copies and independent required-work checks.

No compiler/native launch. Original source files never change. Exact map/pi
values reuse the independent reconciler, not any measured V implementation.
"""
import importlib.util
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
V = HERE.parents[1] / 'v-report-only/20261012'
TRACE_ENV = 'MECH_CRYSTAL_WORK_TRACE'


def independent():
    spec = importlib.util.spec_from_file_location('crystal_independent_work_reconciler', V / 'source_trace.py')
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def insert(source, statement, addition, *, after=False, inside=False):
    expression = re.compile(r'^([ \t]*)' + re.escape(statement) + r'[ \t]*$', re.MULTILINE)
    found = list(expression.finditer(source))
    assert len(found) == 1, ('Nonunique/missing source anchor', statement, len(found))
    match = found[0]
    indent = match.group(1) + ('  ' if inside else '')
    text = '\n'.join(indent + line if line else '' for line in addition.splitlines())
    if after:
        return source[:match.end()] + '\n' + text + source[match.end():]
    return source[:match.start()] + text + '\n' + source[match.start():]


MAP = r'''
def audit_trace_maps(tables, sequence)
  path = ENV["MECH_CRYSTAL_WORK_TRACE"]
  raise "Existing audit path" if File.exists?(path)
  File.open(path, "w") do |audit|
    audit.puts "TRACE_C1|knucleotide|WORK_VARIANT|#{tables.size}|#{sequence.bytesize}"
    tables.each_with_index do |table, index|
      total = 0_i64
      table.each do |key, count|
        audit.puts "MAP|#{LENGTHS[index]}|#{key}|#{count}"
        total += count
      end
      audit.puts "TABLE|#{LENGTHS[index]}|#{table.size}|#{total}"
    end
    audit.puts "END_C1"
  end
end
'''

PI_OPEN = r'''
audit_path = ENV["MECH_CRYSTAL_WORK_TRACE"]
raise "Existing audit path" if File.exists?(audit_path)
audit = File.open(audit_path, "w")
audit.puts "TRACE_C1|pidigits|WORK_VARIANT|#{n}"
audit_candidate = 0
'''.strip()
PI_COMPACT = r'''
audit_candidate += 1
audit.puts "DIV|#{audit_candidate}|lower|#{numerator3}|#{t}|#{digit3}"
audit.puts "DIV|#{audit_candidate}|upper|#{numerator4}|#{t}|#{digit4}"
'''.strip()
PI_OPTIMIZED = r'''
audit_candidate += 1
audit.puts "DIV|#{audit_candidate}|upper|#{upper}|#{denominator}|#{digit4}"
audit.puts "DIV|#{audit_candidate}|lower|#{lower}|#{denominator}|#{digit3}"
'''.strip()

TREE_OPEN = r'''
# Isolated unscored audit only. Retain bounded constructor identities so the GC
# cannot reuse a collected object's address while the forest is reconciled.
AUDIT_PATH = ENV["MECH_CRYSTAL_WORK_TRACE"]
raise "Existing audit path" if File.exists?(AUDIT_PATH)
AUDIT = File.open(AUDIT_PATH, "w")
AUDIT_LOCK = Mutex.new
AUDIT_NODES = [] of Node
AUDIT.puts "TRACE_C1|binarytrees|WORK_VARIANT|#{(ARGV[0]? || "10").to_i}"
'''
TREE_NEW = r'''
AUDIT_LOCK.synchronize do
  AUDIT_NODES << self
  AUDIT.puts "NODE|#{object_id}|#{@left.try(&.object_id) || 0_u64}|#{@right.try(&.object_id) || 0_u64}"
end
'''.strip()
TREE_STRETCH = r'''
root = tree(depth)
result = root.check
AUDIT_LOCK.synchronize { AUDIT.puts "STRETCH|#{root.object_id}|#{depth}|#{result}" }
result
'''.strip()
TREE_REPEATED = r'''
iterations.times do |index|
  root = tree(depth)
  result = root.check
  total += result
  AUDIT_LOCK.synchronize { AUDIT.puts "TREE|#{depth}|#{index}|#{root.object_id}|#{result}" }
end
'''.strip()


def instrument_text(source, benchmark, variant):
    assert benchmark in ('knucleotide', 'pidigits', 'binarytrees') and variant in ('compact', 'optimized')
    assert TRACE_ENV not in source and 'audit_trace_maps' not in source, 'Never reinstrument an audit copy'
    if benchmark == 'knucleotide':
        anchor = '2.times { |index| frequencies(tables[index]) }' if variant == 'compact' else '2.times do |index|'
        source = insert(source, anchor, 'audit_trace_maps(tables, sequence)')
        source += MAP.replace('WORK_VARIANT', variant)
    elif benchmark == 'pidigits':
        source = insert(source, 'n = (ARGV[0]? || "27").to_i', PI_OPEN.replace('WORK_VARIANT', variant), after=True)
        statement = 'digit4 = numerator4 // t' if variant == 'compact' else 'digit3 = lower // denominator'
        source = insert(source, statement, PI_COMPACT if variant == 'compact' else PI_OPTIMIZED, after=True)
        source = insert(source, 'if digit3 == digit4',
                        'audit.puts "ACCEPT|#{audit_candidate}|#{digit3}|#{printed + 1}"', after=True, inside=True)
        source += '\naudit.puts "END_C1"\naudit.close\n'
    else:
        # Declare Node before the audit's Array(Node) constant. The constructor
        # observer body is invoked only after audit state has been initialized.
        source = insert(source, 'def tree(depth : Int32) : Node', TREE_OPEN.replace('WORK_VARIANT', variant).strip())
        source = insert(source, 'def initialize(@left : Node? = nil, @right : Node? = nil)', TREE_NEW, after=True, inside=True)
        assert source.count('  tree(depth).check\n') == 1
        source = source.replace('  tree(depth).check\n', '\n'.join('  ' + line for line in TREE_STRETCH.splitlines()) + '\n')
        assert source.count('  iterations.times { total += tree(depth).check }\n') == 1
        source = source.replace('  iterations.times { total += tree(depth).check }\n', '\n'.join('  ' + line for line in TREE_REPEATED.splitlines()) + '\n')
        source = insert(source, 'long_lived = tree(maximum)',
            'AUDIT_LOCK.synchronize { AUDIT.puts "LONG_START|#{long_lived.object_id}|#{maximum}" }', after=True)
        source = insert(source, r'puts "long lived tree of depth #{maximum}\t check: #{long_lived.check}"',
            'AUDIT_LOCK.synchronize { AUDIT.puts "LONG_END|#{long_lived.object_id}|#{maximum}|#{long_lived.check}" }')
        source += '\nAUDIT.puts "END_C1"\nAUDIT.close\n'
    return source, dict(status='authored-unbuilt-unqualified-trace-copy', scheme='TRACE_C1',
        benchmark=benchmark, variant=variant, original_source_mutated=False,
        preparation_only=True, never_chart_eligible=True, full_workload=False,
        full_performance_run=False, trace_environment=TRACE_ENV,
        observation_only=True, retained_nodes_for_identity_audit=benchmark == 'binarytrees')


def adapted(trace):
    assert trace.startswith(b'TRACE_C1|') and trace.endswith(b'END_C1\n')
    assert trace.count(b'TRACE_C1|') == trace.count(b'END_C1\n') == 1
    return b'TRACE_V1|' + trace[len(b'TRACE_C1|'):-len(b'END_C1\n')] + b'END_V1\n'


def reconcile_maps(trace, data, stdout, variant, *, common_timing_mutex_fd):
    result = independent().reconcile_maps(adapted(trace), data, stdout, variant,
                                         common_timing_mutex_fd=common_timing_mutex_fd)
    return dict(result, language='crystal', actual_trace_scheme='TRACE_C1')


def reconcile_pi(trace, stdout, n, variant, *, common_timing_mutex_fd):
    result = independent().reconcile_pi(adapted(trace), stdout, n, variant,
                                       common_timing_mutex_fd=common_timing_mutex_fd)
    return dict(result, language='crystal', actual_trace_scheme='TRACE_C1')


def reconcile_trees(trace, stdout, n, variant, *, common_timing_mutex_fd):
    independent()._owned_mutex(common_timing_mutex_fd)
    assert type(n) is int and 0 <= n <= 9 and variant in ('compact', 'optimized')
    lines = trace.decode('ascii').splitlines()
    assert lines[0] == f'TRACE_C1|binarytrees|{variant}|{n}'
    assert lines[-1] == 'END_C1' and lines.count('END_C1') == 1
    nodes, roots, stretch, long_start, long_end = {}, {}, [], [], []
    maximum = max(6, n)
    for event in lines[1:-1]:
        parts = event.split('|')
        assert parts[0] in ('NODE', 'TREE', 'STRETCH', 'LONG_START', 'LONG_END')
        values = tuple(int(value) for value in parts[1:])
        if parts[0] == 'NODE':
            assert len(values) == 3
            identity, left, right = values
            assert identity > 0 and identity not in nodes, 'Constructor identity reused/duplicated'
            assert (left == 0) == (right == 0), 'A complete tree cannot have one child'
            assert left == 0 or (left != right and left in nodes and right in nodes), 'Children must be distinct actual earlier allocations'
            nodes[identity] = (left, right)
        elif parts[0] == 'TREE':
            assert len(values) == 4
            depth, index, identity, check = values
            assert (depth, index) not in roots and identity in nodes
            roots[(depth, index)] = (identity, check)
        elif parts[0] == 'STRETCH':
            assert len(values) == 3 and values[0] in nodes
            stretch.append(values)
        elif parts[0] == 'LONG_START':
            assert len(values) == 2 and values[0] in nodes
            long_start.append(values)
        else:
            assert len(values) == 3 and values[0] in nodes
            long_end.append(values)
    assert len(stretch) == len(long_start) == len(long_end) == 1
    assert long_start[0] == long_end[0][:2], 'Long-lived root was replaced or not retained'
    assert stretch[0][1] == maximum + 1 and long_start[0][1] == maximum
    expected_keys = {(depth, index) for depth in range(4, maximum + 1, 2)
                     for index in range(1 << (maximum - depth + 4))}
    assert set(roots) == expected_keys, 'Incomplete/extra ordinary allocated-tree iterations'
    used = set()

    def walk(identity, depth):
        assert identity not in used and identity in nodes, 'Tree/leaf alias or shared allocated subtree'
        used.add(identity)
        left, right = nodes[identity]
        if depth == 0:
            assert left == right == 0, 'Leaf is not an actual distinct Node constructor'
            return 1
        assert left and right
        return 1 + walk(left, depth - 1) + walk(right, depth - 1)

    assert walk(stretch[0][0], maximum + 1) == stretch[0][2]
    assert walk(long_end[0][0], maximum) == long_end[0][2]
    for (depth, index), (identity, check) in sorted(roots.items()):
        assert walk(identity, depth) == check == (1 << (depth + 1)) - 1
    assert used == set(nodes), 'Extra unused allocations or a missing actual root'
    spec = importlib.util.spec_from_file_location('crystal_tree_output_oracle', V / 'references.py')
    references = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(references)
    assert stdout == references.binarytrees(n), 'Trace copy changed complete tree stdout'
    return dict(status='passed', language='crystal', benchmark='binarytrees', variant=variant,
        n=n, actual_nodes=len(nodes), actual_temporary_trees=len(roots),
        all_actual_constructor_edges_reconciled=True, distinct_leaves=True,
        no_shared_subtrees=True, long_lived_root_retained=True,
        stretch_actual_allocation=True, stdout_unchanged=True,
        preparation_only=True, never_chart_eligible=True, full_performance_run=False)
