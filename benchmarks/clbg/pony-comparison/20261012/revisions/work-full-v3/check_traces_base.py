"""Independent bounded-work oracles for diagnostic copies of production sources."""
from collections import Counter, defaultdict


def check(benchmark, lines, n, data, references):
    records = [line.split('\t') for line in lines.decode().splitlines()]
    assert records and all(record and record[0] for record in records)
    threads = {int(record[1]) for record in records if record[0] == 'THREAD'}
    assert 1 <= len(threads) <= 8, ('Observed compute scheduler cap', threads)
    records = [record for record in records if record[0] != 'THREAD']
    proof = dict(complete_work_trace=True, observed_compute_threads=sorted(threads), compute_thread_cap=8,
                 records=len(records), preparation_only=True, score_eligible=False)
    if benchmark == 'binarytrees':
        maximum = max(6, n)
        roots = Counter({maximum + 1: 1, maximum: 1})
        trees = Counter()
        for depth in range(4, maximum + 1, 2):
            trees[depth] = 1 << (maximum - depth + 4)
            roots[depth] += trees[depth]
        expected = Counter()
        for root, count in roots.items():
            for depth in range(root + 1):
                expected[depth] += count * (1 << (root - depth))
        node = Counter(int(record[1]) for record in records if record[0] == 'NODE')
        checks = Counter(int(record[1]) for record in records if record[0] == 'CHECK')
        finals = Counter(int(record[1]) for record in records if record[0] == 'FINAL')
        assert node == checks == expected, ('Every allocated node/leaf is checked', node, checks, expected)
        assert Counter(int(record[1]) for record in records if record[0] == 'TREE') == trees
        assert [record for record in records if record[0] == 'LONG_NEW'] == [['LONG_NEW', str(maximum)]]
        assert [record for record in records if record[0] == 'LONG_CHECK'] == [['LONG_CHECK', str(maximum)]]
        assert sum(finals.values()) > 0 and all(finals[key] <= node[key] for key in finals)
        assert all(record[0] in {'NODE', 'CHECK', 'FINAL', 'TREE', 'LONG_NEW', 'LONG_CHECK'} for record in records)
        proof.update(nodes_by_depth=dict(node), leaf_objects=node[0], check_visits=sum(checks.values()),
                     observed_finalizers_by_depth=dict(finals), allocated_nodes=sum(node.values()),
                     finalizer_completion_fraction=sum(finals.values()) / sum(node.values()), long_root_retained=True)
    elif benchmark == 'knucleotide':
        sequence, _ = references.nucleotide_maps(data)
        widths = references.WIDTHS
        expected_windows = {(width, start): sequence[start:start + width] for width in widths for start in range(max(0, len(sequence) - width + 1))}
        windows, maps, parent, done, handoff = {}, defaultdict(dict), defaultdict(dict), {}, {}
        for record in records:
            width = int(record[1]); assert width in widths
            if record[0] == 'WINDOW':
                key = (width, int(record[2])); assert key not in windows
                windows[key] = record[3]
            elif record[0] in {'MAP', 'PARENT_MAP'}:
                target = maps if record[0] == 'MAP' else parent
                assert record[2] not in target[width]
                target[width][record[2]] = int(record[3])
            else:
                assert record[0] in {'MAP_DONE', 'MAP_PARENT'}
                target = done if record[0] == 'MAP_DONE' else handoff
                assert width not in target
                target[width] = int(record[2])
        expected_maps = {width: dict(Counter(sequence[start:start + width] for start in range(max(0, len(sequence) - width + 1)))) for width in widths}
        assert windows == expected_windows
        assert dict(maps) == dict(parent) == expected_maps, 'Every key/count survives ownership transfer'
        assert done == handoff == {width: len(table) for width, table in expected_maps.items()}
        proof.update(seven_complete_maps=True, windows=len(windows), map_entries=done, ownership_transfer_exact=True)
    elif benchmark == 'pidigits':
        q, r, t, k, printed = 1, 0, 1, 1, 0
        for record in records:
            assert record[0] == 'PI' and printed < n
            numerator3, numerator4 = q * 3 + r, q * 4 + r
            digit3, digit4 = numerator3 // t, numerator4 // t
            assert list(map(int, record[1:])) == [q, r, t, k, numerator3, numerator4, digit3, digit4]
            if digit3 == digit4:
                printed += 1; r = (r - digit3 * t) * 10; q *= 10
            else:
                factor = 2 * k + 1; r = (2 * q + r) * factor; q *= k; t *= factor; k += 1
        assert printed == n
        proof.update(candidate_states=len(records), distinct_numerator_divisions=2 * len(records), digits=printed)
    else:
        raise AssertionError(benchmark)
    return proof
