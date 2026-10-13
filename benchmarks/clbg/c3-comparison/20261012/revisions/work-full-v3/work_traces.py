"""Unscored unique-anchor observers of actual C3 trees, maps and Pi operands."""
from pathlib import Path
from collections import Counter
import json
import re

OPEN = r'''
fn libc::CFile audit_open(char* suffix, char* mode)
{
    char[4096] path;
    char* base = (char*)libc::getenv("CLBG_TRACE");
    if (!base)
    {
        libc::exit(2);
    }
    int length = libc::snprintf(&path[0], path.len, "%s%s", base, suffix);
    if (length < 0 || length >= (int)path.len)
    {
        libc::exit(2);
    }
    libc::CFile result = libc::fopen((ZString)&path[0], (ZString)mode);
    if (!result)
    {
        libc::exit(2);
    }
    return result;
}

fn void audit_close(libc::CFile file)
{
    if (libc::ferror(file) || libc::fclose(file))
    {
        libc::exit(2);
    }
}
'''


def replace_once(source, anchor, replacement):
    assert source.count(anchor) == 1, ('Instrumentation anchor is not unique', anchor)
    return source.replace(anchor, replacement)


def tree_source(source, optimized):
    observer = OPEN + r'''
fn void audit_nodes(libc::CFile file, Node* node)
{
    libc::fprintf(file, "NODE %llu %llu %llu\n", (ulong)(uptr)node,
                  (ulong)(uptr)node.left, (ulong)(uptr)node.right);
    if (node.left)
    {
        audit_nodes(file, node.left);
        audit_nodes(file, node.right);
    }
}

fn void audit_root(libc::CFile file, Node* node, int depth, char* tag, long ordinal)
{
    libc::fprintf(file, "ROOT %s %d %lld %llu\n", tag, depth, ordinal, (ulong)(uptr)node);
    audit_nodes(file, node);
    libc::fprintf(file, "ENDROOT\n");
}
'''
    source = source.replace('import libc;', 'import libc;\nimport std::thread;')
    source = replace_once(source, 'fn Node* make(int depth)', observer + '\nfn Node* make(int depth)')
    source = replace_once(source, '    int depth = math::max(6, (int)libc::atoll(argv[1]));',
        '''    libc::CFile audit_main = audit_open("-main", "w");
    thread::Thread audit_thread = thread::current();
    libc::fprintf(audit_main, "MAIN %llu\\n", (ulong)(uptr)audit_thread.pthread);
    int depth = math::max(6, (int)libc::atoll(argv[1]));''')
    source = replace_once(source, '    Node* stretch = make(depth + 1);',
        '    Node* stretch = make(depth + 1);\n    audit_root(audit_main, stretch, depth + 1, "stretch", 0);')
    source = replace_once(source, '    Node* long_lived = make(depth);',
        '    Node* long_lived = make(depth);\n    audit_root(audit_main, long_lived, depth, "long-open", 0);')
    source = replace_once(source, '    destroy(long_lived);',
        '    audit_root(audit_main, long_lived, depth, "long-close", 0);\n    destroy(long_lived);\n    audit_close(audit_main);')
    if optimized:
        source = replace_once(source, '    Job* job = argument;', r'''    Job* job = argument;
    char[64] audit_suffix;
    libc::snprintf(&audit_suffix[0], audit_suffix.len, "-%d-%lld", job.depth, job.first);
    libc::CFile audit_file = audit_open(&audit_suffix[0], "w");
    thread::Thread audit_thread = thread::current();
    libc::fprintf(audit_file, "RANGE %d %lld %lld %llu\n", job.depth, job.first, job.last,
                  (ulong)(uptr)audit_thread.pthread);''')
        source = replace_once(source, '        Node* tree = make(job.depth);',
            '        Node* tree = make(job.depth);\n        audit_root(audit_file, tree, job.depth, "iteration", iteration);')
        source = replace_once(source, '    job.total = total;', '    job.total = total;\n    audit_close(audit_file);')
        source = replace_once(source, '            threads::join(workers[worker]);',
            '            threads::join(workers[worker]);\n            libc::fprintf(audit_main, "JOIN %d %lld %lld\\n", current, jobs[worker].first, jobs[worker].last);')
    else:
        source = replace_once(source, '            Node* tree = make(current);',
            '            Node* tree = make(current);\n            audit_root(audit_main, tree, current, "iteration", iteration);')
    return source


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


def pi_source(source):
    helper = OPEN + r'''
fn void audit_integer(libc::CFile file, gmp::Mpz* value)
{
    usz size = gmp::size_in_base(value, 10) + 3;
    char* text = libc::malloc(size);
    if (!text)
    {
        libc::exit(2);
    }
    gmp::get_string(text, 10, value);
    libc::fprintf(file, " %s", text);
    libc::free(text);
}

fn void audit_state(libc::CFile file, gmp::Mpz* q, gmp::Mpz* r, gmp::Mpz* t, ulong k, long printed)
{
    libc::fprintf(file, "STATE");
    audit_integer(file, q);
    audit_integer(file, r);
    audit_integer(file, t);
    libc::fprintf(file, " %llu %lld\n", k, printed);
}

fn void audit_divs(libc::CFile file, gmp::Mpz* a, gmp::Mpz* da, gmp::Mpz* b, gmp::Mpz* db)
{
    libc::fprintf(file, "DIV");
    audit_integer(file, a);
    audit_integer(file, da);
    audit_integer(file, b);
    audit_integer(file, db);
    libc::fprintf(file, "\n");
}
'''
    source = replace_once(source, 'fn int main(int argc, char** argv)', helper + '\nfn int main(int argc, char** argv)')
    source = replace_once(source, '    long n = libc::atoll(argv[1]);',
        '    libc::CFile audit_file = audit_open("", "w");\n    long n = libc::atoll(argv[1]);')
    source = replace_once(source, '    while (printed < n)\n    {',
        '    while (printed < n)\n    {\n        audit_state(audit_file, &q, &r, &t, k, printed);')
    source = replace_once(source, '        gmp::divide(&digit4, &numerator4, &t);',
        '        gmp::divide(&digit4, &numerator4, &t);\n        audit_divs(audit_file, &numerator3, &digit3, &numerator4, &digit4);')
    source = replace_once(source, '    gmp::clear(&q);',
        '    libc::fprintf(audit_file, "END %lld\\n", printed);\n    audit_close(audit_file);\n    gmp::clear(&q);')
    return source


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


def map_source(source, optimized):
    helper = OPEN + r'''
fn void audit_owner(int width)
{
    char[32] suffix;
    libc::snprintf(&suffix[0], suffix.len, "-%d", width);
    libc::CFile file = audit_open(&suffix[0], "w");
    thread::Thread owner = thread::current();
    libc::fprintf(file, "OWNER %d %llu\n", width, (ulong)(uptr)owner.pthread);
    audit_close(file);
}

fn void audit_map(Map* counts, int width)
{
    char[32] suffix;
    libc::snprintf(&suffix[0], suffix.len, "-%d", width);
    libc::CFile file = audit_open(&suffix[0], "a");
    thread::Thread reader = thread::current();
    if (counts.allocator.ptr != (void*)&allocators::LIBC_ALLOCATOR)
    {
        libc::exit(2);
    }
    libc::fprintf(file, "MAP %d %lld %lld %llu %llu %llu\n", width, (long)counts.table.len,
                  (long)counts.len(), (ulong)(uptr)reader.pthread,
                  (ulong)(uptr)counts.allocator.ptr, (ulong)(uptr)counts.table.ptr);
    MapIterator entries = counts.iter();
    for (sz i = 0; i < counts.len(); i++)
    {
        MapEntry entry = entries.get(i);
        libc::fprintf(file, "ITEM %llu %lld\n", entry.key, entry.value);
    }
    audit_close(file);
}
'''
    source = source.replace('import libc;', 'import libc;\nimport std::thread;')
    source = replace_once(source, 'fn buffer::Buffer third_record()', helper + '\nfn buffer::Buffer third_record()')
    if optimized:
        source = replace_once(source, '    job.counts.init(&allocators::LIBC_ALLOCATOR);', '    job.counts.init(&allocators::LIBC_ALLOCATOR);\n    audit_owner(job.width);')
    else:
        source = replace_once(source, '    counts.init(&allocators::LIBC_ALLOCATOR);', '    counts.init(&allocators::LIBC_ALLOCATOR);\n    audit_owner(width);')
    source = replace_once(source, '    frequency(&counts[0], 1);', '''    for (int index = 0; index < 7; index++)
    {
        audit_map(&counts[index], WIDTHS[index]);
    }
    frequency(&counts[0], 1);''')
    return source


def map_trace(directory, data, optimized, references, require_growth=False):
    sequence, expected = references.nucleotide_maps(data)
    owner_ids = []
    reader_ids = []
    allocator_ids = []
    table_ids = []
    proof = []
    for width, wanted in zip(references.WIDTHS, expected):
        path = Path(directory) / ('trace-' + str(width))
        lines = path.read_text().splitlines()
        owner = lines[0].split()
        header = lines[1].split()
        assert owner[0] == 'OWNER' and int(owner[1]) == width
        assert header[0] == 'MAP' and int(header[1]) == width
        capacity, length, reader, allocator, table = map(int, header[2:])
        assert allocator and table
        allocator_ids.append(allocator); table_ids.append(table)
        owner_ids.append(int(owner[2])); reader_ids.append(reader)
        assert capacity >= 16 and capacity & (capacity - 1) == 0 and length <= capacity
        actual = {}
        for line in lines[2:]:
            tag, key, count = line.split()
            assert tag == 'ITEM'
            key, count = int(key), int(count)
            word = ''.join('ACGT'[(key >> (2 * shift)) & 3] for shift in range(width - 1, -1, -1))
            assert word not in actual and count > 0
            actual[word] = count
        assert actual == dict(wanted) and len(actual) == length
        windows = max(0, len(sequence) - width + 1)
        assert sum(actual.values()) == windows
        if require_growth and width >= 2:
            assert capacity > 16, 'Actual generic hash table must grow on the growth fixture'
        proof.append(dict(width=width, entries=length, windows=windows, actual_capacity=capacity))
    assert len(set(reader_ids)) == len(set(allocator_ids)) == 1 and reader_ids[0]
    assert len(set(table_ids)) == 7
    if optimized:
        assert len(set(owner_ids)) == 7 and all(owner and owner != reader_ids[0] for owner in owner_ids)
    else:
        assert owner_ids == reader_ids
    return dict(status='passed', actual_seven_complete_maps=True, complete_map_counts_and_window_totals_match=True,
                heap_owned_tables_after_actual_joins=True, source_map_growth=require_growth, maps=proof)


def allocation_llvm(files):
    matches = []
    for path in files:
        text = Path(path).read_text()
        for item in re.finditer(r'^define[^\n]*@(?P<name>[^ (]*binarytrees[^ (]*make)\((?P<parameters>[^\n]*)\)[^\n]*\{\n(?P<body>.*?)^\}', text, re.M | re.S):
            matches.append((Path(path), item['name'], item['parameters'], item['body']))
    assert len(matches) == 1, 'Review ambiguous scored creator IR explicitly'
    path, name, parameters, body = matches[0]
    allocation = re.search(r'call[^\n]*@(?:malloc|calloc)\(', body)
    assert allocation, 'Final original creator lacks real heap allocation'
    first_control = re.search(r'^\s*(?:br|switch|ret)\b', body, re.M)
    assert first_control and allocation.start() < first_control.start(), 'Original leaf path must allocate before any control split'
    depth = re.search(r'i32\s+(?:[A-Za-z]+\s+)*(%[-\w.]+)', parameters)
    assert depth and re.search(r'icmp\s+\w+\s+i32\s+' + re.escape(depth[1]) + r',\s*0', body), 'Original creator must test actual depth'
    children = re.findall(r'call[^\n]*@' + re.escape(name) + r'\(', body)
    assert len(children) >= 2, 'Final original creator lacks both recursive child calls'
    return dict(path=str(path), name=name, real_allocation=True, recursive_calls=len(children))


def allocation_arm(text, name):
    starts = list(re.finditer(r'^([^\n]+):\n', text, re.M))
    matches = []
    for i, start in enumerate(starts):
        if start[1].lstrip('_') == name.lstrip('_'):
            end = starts[i + 1].start() if i + 1 < len(starts) else len(text)
            matches.append(text[start.end():end])
    assert len(matches) == 1, 'Review ambiguous scored ARM creator explicitly'
    body = matches[0]
    # otool renders external calls as a numeric stub plus its authoritative
    # annotation. Numeric recursive calls bind only to this creator's exact
    # entry address; comments and arbitrary targets never prove a callee.
    entry = re.search(r'^([0-9a-fA-F]{8,16})\s+\w+', body, re.M)
    entry_address = int(entry[1], 16) if entry else None
    calls = []
    for match in re.finditer(r'^(?:[0-9a-fA-F]{8,16}\s+)?\s*bl\s+([^\n]+)$', body, re.M):
        operand = match[1].strip()
        direct = re.fullmatch(r'(_?[A-Za-z][\w.$]*)', operand)
        stub = re.fullmatch(r'(0x[0-9a-fA-F]+)\s*;\s*symbol stub for:\s*(_?[A-Za-z][\w.$]*)', operand)
        numeric = re.fullmatch(r'0x[0-9a-fA-F]+', operand)
        if direct:
            callee, rendering, target = direct[1].lstrip('_'), 'direct-symbol', None
        elif stub:
            callee, rendering, target = stub[2].lstrip('_'), 'otool-symbol-stub', int(stub[1], 16)
        elif numeric and entry_address is not None and int(operand, 16) == entry_address:
            callee, rendering, target = name.lstrip('_'), 'exact-creator-entry-address', entry_address
        else:
            callee, rendering, target = None, 'unresolved', None
        calls.append(dict(match=match, callee=callee, rendering=rendering, target=target, original_operand=operand))
    allocators = [call for call in calls if call['callee'] in ('malloc', 'calloc')]
    assert len(allocators) == 1, 'Final native scored creator lacks one actual malloc/calloc'
    allocation = allocators[0]['match']
    child_calls = [call for call in calls if call['callee'] == name.lstrip('_') and
                   call['rendering'] in ('direct-symbol', 'exact-creator-entry-address')]
    children = [call['match'] for call in child_calls]
    assert len(children) >= 2, 'Final native creator lacks both child calls'
    assert allocation.start() < children[0].start()
    preallocation_control = r'\b(?:b(?:\.[a-z]+)?|bl\w*|br\w*|ret\w*|cb\w*|tb\w*)\b'
    assert not re.search(preallocation_control, body[:allocation.start()]), 'Original ARM leaf path splits before allocation'
    between = body[allocation.end():children[0].start()]
    assert re.search(r'\b(?:cbz|cbnz|tbz|tbnz|b\.(?:eq|ne|lt|le|gt|ge|lo|ls|hi|hs))\b', between), 'Review leaf branch before child calls'
    return dict(status='passed', real_allocation_call=True, both_actual_child_calls=True,
                allocation_before_leaf_path=True, scored_native_arm64=True,
                allocator_call={key: value for key, value in allocators[0].items() if key != 'match'},
                creator_entry_address=entry_address,
                recursive_call_resolution=[{key: value for key, value in call.items() if key != 'match'}
                                           for call in child_calls])
