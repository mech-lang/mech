"""Independent synthetic graph/retention/worker-boundary fixture rejection tests."""
from pathlib import Path
import tempfile
import unittest

import tree_work


def graph(address, depth):
    nodes = []
    cursor = address
    def emit(remaining):
        nonlocal cursor
        root = cursor
        cursor += 1
        if remaining:
            left = cursor
            left_lines = emit(remaining - 1)
            right = cursor
            right_lines = emit(remaining - 1)
            return [f'NODE|{root}|{left}|{right}', *left_lines, *right_lines]
        return [f'NODE|{root}|0|0']
    return emit(depth)


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

    def test_worker_iteration_labels_must_match_their_own_section(self):
        def swap(directory):
            one,two=directory/'worker-2.trace',directory/'worker-3.trace'
            one.write_text(one.read_text().replace('ROOT|iteration|4|0|','ROOT|iteration|4|8|'))
            two.write_text(two.read_text().replace('ROOT|iteration|4|8|','ROOT|iteration|4|0|'))
        self.mutate('optimized',swap)

    def test_concurrently_live_churn_graph_is_disjoint_from_retained_graph(self):
        def overlap(directory):
            path=directory/'main-1.trace';lines=path.read_text().splitlines()
            retained=int(next(l for l in lines if l.startswith('ROOT|long-open|')).split('|')[-1])
            begin=next(i for i,l in enumerate(lines) if l.startswith('ROOT|iteration|4|0|'))
            end=next(i for i in range(begin,len(lines)) if lines[i].startswith('TREE_END|'))
            lines[begin:end+1]=[f'ROOT|iteration|4|0|{retained}',*graph(retained,4),'TREE_END|31']
            path.write_text('\n'.join(lines)+'\n')
        self.mutate('compact',overlap)

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
                        good.replace('104 cbz w1, 116', '104 nop'),
                        *[good.replace('100 bl _nimNewObj','098 '+branch+'\n100 bl _nimNewObj') for branch in ('b.eq 116','b 116','br x0','ret','cbnz w1, 116','tbnz w1, 0, 116')]):
            with self.assertRaises(AssertionError):
                tree_work.creator_arm(changed, 'make__compact_u7')

    def test_actual_compact_exception_target_is_null_and_other_returns_allocate(self):
        cohort = Path(__file__).resolve().parents[2]
        source = (cohort / 'work-qualification/work-v3/nim-binarytrees-compact/scored-creator.c').read_text()
        arm = (cohort.parents[1] / 'continuation/jobs/nim-tree-arm-inspection-v1/compact-scored-creator.arm').read_text()
        result = tree_work.creator_arm(arm, 'make__compact_u7', source)
        self.assertTrue(result['allocation_before_all_nonexception_returns'])
        self.assertFalse(result['unconditional_allocation_before_control'])
        self.assertEqual(result['preallocation_exception_guard']['status'], 'matched-reviewed-runtime-exception-propagation')
        self.assertEqual(result['preallocation_exception_guard']['literal_NULL_only_epilogue'][-1], 'ret')
        self.assertEqual(len(result['nonexception_return_addresses']), 3)
        optimized = cohort / 'allocation-inspections/optimized-v1'
        optimized_source = (optimized / 'optimized-scored-creator.c').read_text()
        optimized_arm = (optimized / 'optimized-scored-creator.arm').read_text()
        separate = tree_work.creator_arm(optimized_arm, 'make__optimized_u17', optimized_source)
        self.assertTrue(separate['allocation_before_all_nonexception_returns'])
        self.assertFalse(separate['unconditional_allocation_before_control'])
        self.assertEqual(separate['preallocation_exception_guard']['status'], 'matched-reviewed-runtime-exception-propagation')
        self.assertGreater(len(separate['nonexception_return_addresses']), 0)

    def test_exception_exemption_rejects_depth_shortcut_unknown_call_and_nonnull_return(self):
        cohort = Path(__file__).resolve().parents[2]
        source = (cohort / 'work-qualification/work-v3/nim-binarytrees-compact/scored-creator.c').read_text()
        original = (cohort.parents[1] / 'continuation/jobs/nim-tree-arm-inspection-v1/compact-scored-creator.arm').read_text()
        for mutation in ('depth', 'tls-target', 'nonnull-error', 'outside-target', 'no-allocator', 'source-guard'):
            arm, creator = original, source
            if mutation == 'depth': arm = arm.replace('tbnz\tw8, #0x0,', 'tbnz\tw21, #0x0,', 1)
            elif mutation == 'tls-target': arm = arm.replace('blr\tx8', 'blr\tx9', 1)
            elif mutation == 'nonnull-error': arm = arm.replace('mov\tx19, #0x0', 'mov\tx19, #0x1', 1)
            elif mutation == 'outside-target': arm = arm.replace('0x10000b0c8', '0xdeadbeef', 1)
            elif mutation == 'no-allocator': arm = arm.replace('bl\t_nimNewObj', 'bl\t_unknownHelper', 1)
            else: creator = creator.replace('NIM_UNLIKELY(*nimErr_)', 'depth_p0 == 0', 1)
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                tree_work.creator_arm(arm, 'make__compact_u7', creator)


if __name__ == '__main__':
    unittest.main(verbosity=2)
