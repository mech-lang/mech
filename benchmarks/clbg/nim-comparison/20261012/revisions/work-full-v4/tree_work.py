"""Observe every allocated live tree, disjoint worker intervals and retained root."""
import hashlib
import re
from pathlib import Path

SUPPORT = '''import std/[os, typedthreads]
createDir(getEnv("MECH_NIM_TREE_TRACE"))
proc auditOpen*(label: string): File =
  result = open(getEnv("MECH_NIM_TREE_TRACE") & "/" & label & "-" & $getThreadId() & ".trace", fmAppend)
  result.writeLine("BEGIN|", label, "|", getThreadId())
'''

OBSERVER = '''proc auditTree(audit: File, node: Node, depth: int, kind: string, index: int) =
  audit.writeLine("ROOT|", kind, "|", depth, "|", index, "|", cast[uint](node))
  var seen = initHashSet[uint]()
  proc visit(current: Node) =
    doAssert current != nil
    let address = cast[uint](current)
    doAssert address notin seen
    seen.incl(address)
    audit.writeLine("NODE|", address, "|", cast[uint](current.left), "|", cast[uint](current.right))
    if current.left != nil:
      doAssert current.right != nil
      visit(current.left)
      visit(current.right)
    else:
      doAssert current.right == nil
  visit(node)
  audit.writeLine("TREE_END|", seen.len)
'''


def instrument(source, variant):
    assert 'MECH_NIM_TREE_TRACE' not in source and 'auditTree' not in source
    original = source
    edits = []

    def change(before, after):
        nonlocal source
        assert source.count(before) == 1, ('Unique tree audit anchor', before)
        source = source.replace(before, after)
        edits.append(dict(before=before, after=after))

    source = 'import std/sets\nimport audit_support\n' + source
    change('proc check(node: Node): int =\n', OBSERVER + '\nproc check(node: Node): int =\n')
    change('let depth = max(6, parseInt(paramStr(1)))\n',
           'let audit = auditOpen("main")\nlet depth = max(6, parseInt(paramStr(1)))\n')
    change('  let stretch = make(depth + 1)\n',
           '  let stretch = make(depth + 1)\n  auditTree(audit, stretch, depth + 1, "stretch", -1)\n')
    change('let longLived = make(depth)\n',
           'let longLived = make(depth)\nauditTree(audit, longLived, depth, "long-open", -1)\n')
    last = 'echo "long lived tree of depth ", depth, "\\t check: ", check(longLived)\n'
    change(last, 'auditTree(audit, longLived, depth, "long-close", -1)\n' + last)
    if variant == 'compact':
        change('    let tree = make(current)\n',
               '    let tree = make(current)\n    auditTree(audit, tree, current, "iteration", iteration)\n')
    else:
        change('proc trees(job: Job) {.thread.} =\n', '''proc trees(job: Job) {.thread.} =
  let audit = auditOpen("worker")
  defer:
    audit.writeLine("END")
    audit.close()
  audit.writeLine("RANGE|", job.depth, "|", job.first, "|", job.last)
''')
        change('    let tree = make(job.depth)\n',
               '    let tree = make(job.depth)\n    auditTree(audit, tree, job.depth, "iteration", iteration)\n')
        change('    joinThread(threads[worker])\n',
               '    joinThread(threads[worker])\n    audit.writeLine("JOIN|", current, "|", worker)\n')
    source += '\naudit.writeLine("END")\naudit.close()\n'
    return source, dict(scored_source_sha256=hashlib.sha256(original.encode()).hexdigest(),
        trace_source_sha256=hashlib.sha256(source.encode()).hexdigest(), changes=edits,
        preserved_make_and_check=True, workload_helpers_replaced=False,
        observations='Temporary integer addresses only; no retained Node references after observer returns')


def reconcile(directory, n, variant):
    maximum = max(6, n)
    roots, ranges, joins, tids, sections = [], [], [], set(), 0
    main_tids = set()
    files = sorted(Path(directory).glob('*.trace'))
    assert files
    for path in files:
        open_section = False
        current_root = None
        section_range = None
        section_iterations = []
        main_order = []
        for line in path.read_text().splitlines():
            fields = line.split('|')
            event = fields[0]
            if event == 'BEGIN':
                assert not open_section and current_root is None and len(fields) == 3
                open_section = True
                sections += 1
                label, tid = fields[1], int(fields[2])
                assert label in ('main', 'worker') and tid > 0
                (main_tids if label == 'main' else tids).add(tid)
                section_range = None
                section_iterations = []
            elif event == 'END':
                assert open_section and current_root is None and len(fields) == 1
                if label == 'worker':
                    assert section_range is not None
                    d, first, last = section_range
                    assert section_iterations == [(d,i) for i in range(first,last)], 'Observed iteration labels must match their own actual worker interval'
                open_section = False
            elif event == 'RANGE':
                assert open_section and label == 'worker' and current_root is None and len(fields) == 4
                assert section_range is None and section_iterations == []
                section_range = tuple(map(int, fields[1:]))
                ranges.append(section_range)
            elif event == 'JOIN':
                assert open_section and label == 'main' and current_root is None and len(fields) == 3
                joins.append(tuple(map(int, fields[1:])))
                main_order.append('JOIN')
            elif event == 'ROOT':
                assert open_section and current_root is None and len(fields) == 5
                current_root = dict(kind=fields[1], depth=int(fields[2]), index=int(fields[3]),
                    root=int(fields[4]), nodes={}, label=label, prior_joins=joins.copy())
                assert current_root['root'] > 0
            elif event == 'NODE':
                assert open_section and current_root is not None and len(fields) == 4
                address, left, right = map(int, fields[1:])
                assert address > 0 and address not in current_root['nodes']
                assert (left == 0) == (right == 0)
                current_root['nodes'][address] = (left, right)
            else:
                assert event == 'TREE_END' and open_section and current_root is not None and len(fields) == 2
                nodes = current_root['nodes']
                depth = current_root['depth']
                assert int(fields[1]) == len(nodes) == (1 << (depth + 1)) - 1
                reached = set()

                def visit(address, remaining):
                    assert address in nodes and address not in reached
                    reached.add(address)
                    left, right = nodes[address]
                    if remaining == 0:
                        assert left == right == 0
                    else:
                        assert left > 0 and right > 0 and left != right
                        visit(left, remaining - 1)
                        visit(right, remaining - 1)

                visit(current_root['root'], depth)
                assert reached == set(nodes)
                assert sum(left == 0 for left, _ in nodes.values()) == 1 << depth
                roots.append(current_root)
                if current_root['kind'] == 'iteration':
                    section_iterations.append((current_root['depth'],current_root['index']))
                if label == 'main': main_order.append(current_root['kind'])
                current_root = None
        assert not open_section and current_root is None
        if main_order:
            assert main_order[:2] == ['stretch','long-open'] and main_order[-1:] == ['long-close']
            assert all(k == ('JOIN' if variant == 'optimized' else 'iteration') for k in main_order[2:-1])
    assert len(main_tids) == 1
    assert not tids & main_tids
    expected_kinds = {'stretch': 1, 'long-open': 1, 'long-close': 1}
    for kind, count in expected_kinds.items():
        assert sum(root['kind'] == kind for root in roots) == count
    stretch = next(root for root in roots if root['kind'] == 'stretch')
    assert stretch['depth'] == maximum + 1 and stretch['index'] == -1 and stretch['label'] == 'main'
    retained = [next(root for root in roots if root['kind'] == kind) for kind in ('long-open', 'long-close')]
    assert retained[0]['depth'] == retained[1]['depth'] == maximum
    assert retained[0]['root'] == retained[1]['root'] and retained[0]['nodes'] == retained[1]['nodes']
    assert all(root['kind'] in set(expected_kinds) | {'iteration'} for root in roots)
    retained_nodes = set(retained[0]['nodes'])
    assert all(not (set(root['nodes']) & retained_nodes) for root in roots if root['kind']=='iteration'), 'Concurrently live churn graph overlaps retained long-lived graph'
    # Stretch is already released; earlier allocator addresses may validly recur.
    assert len(roots) == 3 + sum(1 << (maximum - depth + 4) for depth in range(4, maximum + 1, 2))
    for depth in range(4, maximum + 1, 2):
        iterations = 1 << (maximum - depth + 4)
        actual = [root for root in roots if root['kind'] == 'iteration' and root['depth'] == depth]
        assert len(actual) == iterations and {root['index'] for root in actual} == set(range(iterations))
        if variant == 'optimized':
            count = min(8, iterations)
            expected_ranges = [(depth, iterations * worker // count, iterations * (worker + 1) // count) for worker in range(count)]
            assert sorted(r for r in ranges if r[0] == depth) == expected_ranges
            assert sorted(j for j in joins if j[0] == depth) == [(depth, worker) for worker in range(count)]
            assert all((depth, worker) in retained[1]['prior_joins'] for worker in range(count))
            assert all(root['label'] == 'worker' for root in actual)
        else:
            assert all(root['label'] == 'main' for root in actual)
    if variant == 'compact':
        assert ranges == joins == [] and tids == set() and sections == 1
    else:
        assert tids and sections == 1 + len(ranges)
        assert len(ranges) == len(joins) == 8 * len(range(4, maximum + 1, 2))
    trace_pins = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
    return dict(status='passed', distinct_nodes_and_all_leaves_per_actual_root=True,
        no_closed_form_substitute=True, actual_stretch_and_all_iteration_roots=True,
        unchanged_long_lived_root_after_churn=True, actual_joined_worker_intervals=True,
        actual_interval_labels_correlated=True, concurrent_churn_disjoint_from_retained_graph=True,
        maximum_simultaneous_compute_workers=8 if variant == 'optimized' else 1,
        roots_observed=len(roots), allocated_nodes_observed=sum(len(root['nodes']) for root in roots if root['kind'] != 'long-close'),
        leaf_nodes_observed=sum(sum(left == 0 for left, _ in root['nodes'].values()) for root in roots if root['kind'] != 'long-close'),
        trace_pins=trace_pins, no_gc_pressure_or_scored_timing_claim=True,
        preparation_only=True, never_chart_eligible=True)


def creator_c(path):
    text = Path(path).read_text()
    match = re.search(r'N_LIB_PRIVATE N_NIMCALL\([^\n]+, (make__\w+)\)\(NI depth_p0\) \{', text)
    assert match, 'Actual generated creator function missing'
    name = match.group(1)
    start = match.end() - 1
    balance = 0
    end = None
    for offset in range(start, len(text)):
        if text[offset] == '{':
            balance += 1
        elif text[offset] == '}':
            balance -= 1
            if balance == 0:
                end = offset + 1
                break
    assert end is not None
    body = text[start:end]
    assert body.count('nimNewObj(') == 1 and body.count(name + '(') == 2
    assert '(*result).left' in body and '(*result).right' in body
    assert body.index('nimNewObj(') < body.index('depth_p0'), 'Leaf allocation must precede depth branch'
    return name, body


def creator_arm(text, name, source=None):
    marker = '_' + name + ':'
    assert text.count(marker) == 1
    after = text.split(marker, 1)[1]
    boundary = re.search(r'(?m)^_[A-Za-z][A-Za-z0-9_$]*:\s*$', after)
    body = after[:boundary.start()] if boundary else after
    allocation = re.search(r'\bbl\s+_nimNewObj\b', body)
    assert allocation, 'Final ARM creator must allocate'
    import arm_allocation
    dominance = arm_allocation.allocation_dominance(body, name, source)
    children = list(re.finditer(r'\bbl\s+_' + re.escape(name) + r'\b', body))
    assert len(children) == 2, 'Final ARM creator must call both child paths'
    between = body[allocation.end():children[0].start()]
    assert re.search(r'\b(?:cbz|cbnz|tbz|tbnz|b\.(?:eq|ne|lt|le|gt|ge|hi|ls|hs|lo|mi|pl|vs|vc))\b', between), 'Final creator must retain conditional leaf path after allocation and before child calls'
    return dict(dominance, creator_symbol=name, real_allocation_call=True,
        two_actual_child_calls=True, final_arm_leaf_branch=True,
        creator_disassembly_sha256=hashlib.sha256(body.encode()).hexdigest())
