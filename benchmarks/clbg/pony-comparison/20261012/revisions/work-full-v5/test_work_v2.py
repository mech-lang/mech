"""Meaningful graph/lifecycle fixtures; unexecuted until reviewed locked packet."""
import copy
import unittest
from check_traces import tree_identity


def actual_fixture():
    records, graph, next_id = [], {}, 1
    def event(*items): records.append(list(map(str, items)))
    def build(depth):
        nonlocal next_id
        node = next_id; next_id += 1
        event('NODE', depth)
        left, right = (build(depth - 1), build(depth - 1)) if depth else (0, 0)
        graph[node] = (depth, left, right)
        event('GRAPH', depth, node, left, right)
        return node
    def check(node):
        depth, left, right = graph[node]
        event('CHECK', depth, node)
        if depth: check(left); check(right)
    def retire(node):
        depth, left, right = graph[node]
        if depth: retire(left); retire(right)
        event('FINAL', depth, node)
    stretch = build(7); event('ROOT_STRETCH_NEW', 7, stretch)
    check(stretch); event('ROOT_STRETCH_CHECK', 7, stretch); retire(stretch)
    long = build(6); event('ROOT_LONG_NEW', 6, long)
    for depth in (4, 6):
        for _ in range(1 << (6 - depth + 4)):
            root = build(depth); event('ROOT_CHURN_NEW', depth, root, 1)
            check(root); event('ROOT_CHURN_CHECK', depth, root, 1); retire(root)
        event('WORKER_DONE', depth, 100000 + depth)
        event('WORKER_JOIN', depth, 100000 + depth)
    check(long); event('ROOT_LONG_CHECK', 6, long); retire(long)
    return records


class TreeProofV2(unittest.TestCase):
    def test_complete_graph_and_lifecycle(self):
        proof = tree_identity(actual_fixture(), 6, 'compact')
        self.assertEqual(proof['long_graph_nodes'], 127)
        self.assertTrue(proof['long_graph_retained_across_all_worker_joins'])

    def test_early_retained_node_finalizer_rejected(self):
        records = actual_fixture()
        root = next(row for row in records if row[0] == 'ROOT_LONG_NEW')
        records.insert(records.index(root) + 1, ['FINAL', root[1], root[2]])
        with self.assertRaises(AssertionError): tree_identity(records, 6, 'compact')

    def test_shared_leaf_rejected(self):
        records = actual_fixture()
        row = next(row for row in records if row[0] == 'GRAPH' and row[1] == '1')
        row[4] = row[3]
        with self.assertRaises(AssertionError): tree_identity(records, 6, 'compact')

    def test_missing_worker_join_rejected(self):
        records = actual_fixture()
        records.remove(next(row for row in records if row[0] == 'WORKER_JOIN'))
        with self.assertRaises(AssertionError): tree_identity(records, 6, 'compact')

    def test_skipped_actual_check_rejected(self):
        records = actual_fixture()
        records.remove(next(row for row in records if row[0] == 'CHECK' and row[1] == '0'))
        with self.assertRaises(AssertionError): tree_identity(records, 6, 'compact')

    def test_live_identity_reuse_rejected(self):
        records = actual_fixture()
        row = next(row for row in records if row[0] == 'GRAPH')
        records.insert(records.index(row) + 1, copy.deepcopy(row))
        with self.assertRaises(AssertionError): tree_identity(records, 6, 'compact')

    def test_released_stretch_identity_reuse_is_allowed(self):
        records = actual_fixture()
        released = next(row[2] for row in records if row[0] == 'GRAPH' and row[1] == '0')
        after = next(index for index, row in enumerate(records) if row[0] == 'ROOT_STRETCH_CHECK')
        replacement = next(row[2] for row in records[after:] if row[0] == 'GRAPH' and row[1] == '0')
        for row in records:
            positions = range(2, 5) if row[0] == 'GRAPH' else (2,) if row[0] in {'CHECK', 'FINAL'} else ()
            for index in positions:
                if row[index] == replacement: row[index] = released
        self.assertTrue(tree_identity(records, 6, 'compact')['long_graph_retained_across_all_worker_joins'])


if __name__ == '__main__': unittest.main()
