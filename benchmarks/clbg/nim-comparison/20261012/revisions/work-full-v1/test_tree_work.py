"""Independent synthetic graph/retention/worker-boundary fixture rejection tests."""
from pathlib import Path
import tempfile
import unittest

import tree_work


def graph(address, depth):
    nodes = []

    def emit(root, remaining):
        left, right = (root * 2, root * 2 + 1) if remaining else (0, 0)
        nodes.append(f'NODE|{root}|{left}|{right}')
        if remaining:
            emit(left, remaining - 1)
            emit(right, remaining - 1)

    emit(address, depth)
    return nodes


def fixture(directory, n, variant):
    maximum = max(6, n)
    main = ['BEGIN|main|1']
    counter = 10000

    def root(lines, kind, depth, index, address=None):
        nonlocal counter
        counter += 10000
        address = address or counter
        lines.append(f'ROOT|{kind}|{depth}|{index}|{address}')
        lines.extend(graph(address, depth))
        lines.append(f'TREE_END|{(1 << (depth + 1)) - 1}')
        return address

    root(main, 'stretch', maximum + 1, -1)
    retained = root(main, 'long-open', maximum, -1)
    for depth in range(4, maximum + 1, 2):
        count = 1 << (maximum - depth + 4)
        if variant == 'compact':
            for iteration in range(count):
                root(main, 'iteration', depth, iteration)
        else:
            for worker in range(8):
                first, last = count * worker // 8, count * (worker + 1) // 8
                lines = [f'BEGIN|worker|{worker + 2}', f'RANGE|{depth}|{first}|{last}']
                for iteration in range(first, last):
                    root(lines, 'iteration', depth, iteration)
                lines.append('END')
                with (directory / f'worker-{worker + 2}.trace').open('a') as stream:
                    stream.write('\n'.join(lines) + '\n')
                main.append(f'JOIN|{depth}|{worker}')
    root(main, 'long-close', maximum, -1, retained)
    main.append('END')
    (directory / 'main-1.trace').write_text('\n'.join(main) + '\n')


class ActualGraphFixtures(unittest.TestCase):
    def test_minimum_odd_and_uneven_depth_shapes(self):
        for n in (0, 4, 7, 9):
            for variant in ('compact', 'optimized'):
                with tempfile.TemporaryDirectory() as temporary:
                    directory = Path(temporary)
                    fixture(directory, n, variant)
                    result = tree_work.reconcile(directory, n, variant)
                    self.assertTrue(result['distinct_nodes_and_all_leaves_per_actual_root'])
                    self.assertTrue(result['unchanged_long_lived_root_after_churn'])

    def mutate(self, variant, action):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            fixture(directory, 0, variant)
            action(directory)
            with self.assertRaises(AssertionError):
                tree_work.reconcile(directory, 0, variant)

    def test_missing_duplicate_and_reused_node_addresses_rejected(self):
        def missing(directory):
            path = directory / 'main-1.trace'
            lines = path.read_text().splitlines()
            lines.pop(3)
            path.write_text('\n'.join(lines) + '\n')

        def reused(directory):
            path = directory / 'main-1.trace'
            lines = path.read_text().splitlines()
            first = lines[2].split('|')
            first[3] = first[2]
            lines[2] = '|'.join(first)
            path.write_text('\n'.join(lines) + '\n')

        self.mutate('compact', missing)
        self.mutate('compact', reused)

    def test_changed_retained_root_rejected(self):
        def change(directory):
            path = directory / 'main-1.trace'
            lines = path.read_text().splitlines()
            index = next(i for i, line in enumerate(lines) if line.startswith('ROOT|long-close|'))
            fields = lines[index].split('|')
            fields[-1] = '777'
            lines[index] = '|'.join(fields)
            path.write_text('\n'.join(lines) + '\n')

        self.mutate('compact', change)

    def test_overlapping_worker_ranges_and_missing_joins_rejected(self):
        def overlap(directory):
            path = directory / 'worker-3.trace'
            path.write_text(path.read_text().replace('RANGE|4|8|16', 'RANGE|4|0|16'))

        def missing_join(directory):
            path = directory / 'main-1.trace'
            path.write_text(path.read_text().replace('JOIN|4|0\n', ''))

        self.mutate('optimized', overlap)
        self.mutate('optimized', missing_join)

    def test_instrumentation_retains_original_creators_checks_and_observes_before_check(self):
        cohort = Path(__file__).resolve().parents[2]
        for variant in ('compact', 'optimized'):
            source = (cohort / 'sources/binarytrees' / (variant + '.nim')).read_text()
            changed, evidence = tree_work.instrument(source, variant)
            original_helpers = source.split('proc make(', 1)[1].split('let depth =', 1)[0]
            self.assertIn('new result', original_helpers)
            self.assertEqual(changed.count('new result'), 1)
            self.assertEqual(changed.count('result.left = make(depth - 1)'), 1)
            self.assertEqual(changed.count('result.right = make(depth - 1)'), 1)
            self.assertEqual(changed.count('total += check(tree)'), 1)
            self.assertLess(changed.index('auditTree(audit, tree,'), changed.index('total += check(tree)'))
            self.assertTrue(evidence['preserved_make_and_check'])

    def test_final_scored_creator_requires_real_allocation_and_both_children(self):
        good = '''_make__compact_u7:
100 bl _nimNewObj
104 cbz w1, 116
108 bl _make__compact_u7
112 bl _make__compact_u7
116 ret
_next:
120 ret
'''
        result = tree_work.creator_arm(good, 'make__compact_u7')
        self.assertTrue(result['real_allocation_call'])
        for changed in (good.replace('bl _nimNewObj', 'mov x0, xzr'),
                        good.replace('112 bl _make__compact_u7', '112 nop'),
                        good.replace('104 cbz w1, 116', '104 nop')):
            with self.assertRaises(AssertionError):
                tree_work.creator_arm(changed, 'make__compact_u7')


if __name__ == '__main__':
    unittest.main(verbosity=2)
