"""Unexecuted v2 graph/lifecycle oracle; depth/count oracle stays independent."""
from collections import Counter
from check_traces_base import check as base_check

BASE = {'THREAD', 'NODE', 'CHECK', 'FINAL', 'TREE', 'LONG_NEW', 'LONG_CHECK'}


def tree_identity(records, n, variant):
    maximum = max(6, n)
    latest, long_graph, active, completed = {}, None, {}, Counter()
    done, joined, visits = set(), set(), Counter()
    long_identity = None
    stretch_checked = long_checked = False

    def graph(identity, depth):
        found = set()
        def visit(node, level):
            assert node > 0 and node not in found, 'Missing/shared/cyclic node identity'
            found.add(node)
            item = latest[node]
            assert not item['finalized'] and item['depth'] == level
            if level == 0:
                assert item['children'] == (0, 0), 'Leaf must be an actual distinct object'
            else:
                for child in item['children']:
                    visit(child, level - 1)
        visit(identity, depth)
        assert len(found) == (1 << (depth + 1)) - 1
        return frozenset((node, latest[node]['epoch']) for node in found)

    def alive(nodes):
        return all(latest[node]['epoch'] == epoch and not latest[node]['finalized'] for node, epoch in nodes)

    for record in records:
        kind = record[0]
        if kind == 'GRAPH':
            depth, identity, left, right = map(int, record[1:])
            prior = latest.get(identity)
            assert identity > 0 and (prior is None or prior['finalized']), 'Live graph address/digest reused'
            latest[identity] = dict(depth=depth, children=(left, right), finalized=False,
                                    epoch=0 if prior is None else prior['epoch'] + 1)
        elif kind == 'FINAL':
            depth, identity = map(int, record[1:])
            item = latest[identity]
            assert item['depth'] == depth and not item['finalized']
            assert not any((identity, item['epoch']) in nodes for nodes in active.values()), 'Active graph finalized'
            if long_graph is not None and not long_checked:
                assert (identity, item['epoch']) not in long_graph, 'Long graph finalized before final check/joins'
            item['finalized'] = True
        elif kind == 'ROOT_STRETCH_NEW':
            depth, identity = map(int, record[1:])
            assert depth == maximum + 1 and 'stretch' not in active
            active['stretch'] = graph(identity, depth)
        elif kind == 'ROOT_STRETCH_CHECK':
            depth, identity = map(int, record[1:])
            nodes = active.pop('stretch')
            assert depth == maximum + 1 and nodes == graph(identity, depth)
            assert all(visits[item] == 1 for item in nodes), 'Every stretch node checked exactly once'
            stretch_checked = True
        elif kind == 'ROOT_LONG_NEW':
            depth, identity = map(int, record[1:])
            assert stretch_checked and depth == maximum and long_graph is None
            long_identity, long_graph = identity, graph(identity, depth)
        elif kind == 'ROOT_CHURN_NEW':
            depth, identity, thread = map(int, record[1:])
            assert long_graph is not None and alive(long_graph) and thread not in active
            nodes = graph(identity, depth)
            assert nodes.isdisjoint(long_graph)
            assert all(nodes.isdisjoint(other) for other in active.values()), 'Concurrent trees share nodes'
            active[thread] = nodes
        elif kind == 'ROOT_CHURN_CHECK':
            depth, identity, thread = map(int, record[1:])
            nodes = active.pop(thread)
            assert alive(long_graph) and nodes == graph(identity, depth)
            assert all(visits[item] == 1 for item in nodes), 'Every churn node checked exactly once'
            completed[depth] += 1
        elif kind in {'WORKER_DONE', 'WORKER_JOIN'}:
            depth, identity = map(int, record[1:])
            key = (depth, identity)
            assert long_graph is not None and alive(long_graph) and not long_checked
            if kind == 'WORKER_DONE':
                assert key not in done
                done.add(key)
            else:
                assert key in done and key not in joined
                joined.add(key)
        elif kind == 'CHECK':
            depth, identity = map(int, record[1:])
            assert latest[identity]['depth'] == depth and not latest[identity]['finalized']
            visits[(identity, latest[identity]['epoch'])] += 1
        elif kind == 'ROOT_LONG_CHECK':
            depth, identity = map(int, record[1:])
            assert identity == long_identity and depth == maximum and not long_checked
            assert not active and done == joined and alive(long_graph)
            assert graph(identity, depth) == long_graph
            assert all(visits[item] == 1 for item in long_graph), 'Every original long node checked exactly once'
            long_checked = True
        else:
            assert kind in BASE, ('Unknown tree event', kind)
    expected = {depth: 1 << (maximum - depth + 4) for depth in range(4, maximum + 1, 2)}
    assert completed == expected and long_checked and stretch_checked
    counts = Counter(depth for depth, _ in joined)
    assert counts == {depth: 1 if variant == 'compact' else min(8, count) for depth, count in expected.items()}
    assert done == joined and not active
    return dict(actual_graph_identities=True, concurrent_graphs_disjoint=True,
                long_graph_retained_across_all_worker_joins=True, long_root_identity=long_identity,
                long_graph_nodes=len(long_graph), churn_roots=dict(completed),
                worker_done_records=len(done), worker_join_records=len(joined),
                released_graph_identity_reuse_allowed=True)


def check(benchmark, lines, n, data, references, variant=None):
    if benchmark != 'binarytrees':
        return base_check(benchmark, lines, n, data, references)
    assert variant in {'compact', 'optimized'}
    records = [line.split('\t') for line in lines.decode().splitlines()]
    identities = tree_identity(records, n, variant)
    scalar = []
    for record in records:
        if record[0] not in BASE: continue
        scalar.append('\t'.join(record[:2] if record[0] in {'CHECK', 'FINAL'} else record))
    proof = base_check(benchmark, ('\n'.join(scalar) + '\n').encode(), n, data, references)
    proof.update(identities, long_root_retained=True,
                 scored_binary_allocation_review_still_required=True)
    return proof
