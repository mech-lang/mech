"""Meaningful observer-corruption fixtures; never execute native programs."""
from collections import Counter
from pathlib import Path
import tempfile
import re
import unittest

import references
import work_traces as traces


class PiChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
    def tearDown(self):
        self.temporary.cleanup()
    def test_real_accepted_and_rejected_candidates_and_zero(self):
        (self.directory / 'trace').write_text('STATE 1 0 1 1 0\nDIV 3 3 4 4\nSTATE 1 6 3 2 0\nDIV 9 3 10 3\nEND 1\n')
        result = traces.pi_trace(self.directory, 1)
        self.assertEqual((result['accepted_candidates'], result['rejected_candidates']), (1, 1))
        (self.directory / 'trace').write_text('END 0\n')
        self.assertEqual(traces.pi_trace(self.directory, 0)['accepted_candidates'], 0)
    def test_wrong_actual_second_division_rejected(self):
        (self.directory / 'trace').write_text('STATE 1 0 1 1 0\nDIV 3 3 4 3\nEND 1\n')
        with self.assertRaises(AssertionError):
            traces.pi_trace(self.directory, 1)


class MapChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.data = b'>THREE\nACGTACGT\n'
        _, self.maps = references.nucleotide_maps(self.data)
        for index, (width, counts) in enumerate(zip(references.WIDTHS, self.maps)):
            records = [f'OWNER {width} {index + 100}', f'MAP {width} 16 {len(counts)} 99 500 {1000 + index}']
            for word, count in counts.items():
                key = 0
                for value in word:
                    key = (key << 2) | 'ACGT'.index(value)
                records.append(f'ITEM {key} {count}')
            (self.directory / ('trace-' + str(width))).write_text('\n'.join(records) + '\n')
    def tearDown(self):
        self.temporary.cleanup()
    def test_all_real_tables_and_tail_windows(self):
        proof = traces.map_trace(self.directory, self.data, True, references)
        self.assertEqual([row['windows'] for row in proof['maps']], [8, 7, 6, 5, 3, 0, 0])
    def test_missing_overlapping_window_rejected(self):
        path = self.directory / 'trace-2'
        path.write_text(path.read_text().replace('ITEM 1 2', 'ITEM 1 1'))
        with self.assertRaises(AssertionError):
            traces.map_trace(self.directory, self.data, True, references)
    def test_main_thread_substituted_for_map_owner_rejected(self):
        path = self.directory / 'trace-18'
        path.write_text(path.read_text().replace('OWNER 18 106', 'OWNER 18 99'))
        with self.assertRaises(AssertionError):
            traces.map_trace(self.directory, self.data, True, references)
    def test_fixed_capacity_on_growth_fixture_rejected(self):
        with self.assertRaisesRegex(AssertionError, 'grow'):
            traces.map_trace(self.directory, self.data, True, references, require_growth=True)


def root_record(tag, depth, ordinal, address):
    lines = [f'ROOT {tag} {depth} {ordinal} {address}']
    cursor = address
    def build(remaining):
        nonlocal cursor
        node = cursor
        cursor += 1
        if remaining:
            left, left_lines = build(remaining - 1)
            right, right_lines = build(remaining - 1)
            return node, [f'NODE {node} {left} {right}', *left_lines, *right_lines]
        return node, [f'NODE {node} 0 0']
    root, nodes = build(depth)
    return [*lines, *nodes, 'ENDROOT']


class TreeChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        lines = ['MAIN 99', *root_record('stretch', 7, 0, 1000), *root_record('long-open', 6, 0, 2000)]
        address = 3000
        for depth in (4, 6):
            for ordinal in range(1 << (6 - depth + 4)):
                lines += root_record('iteration', depth, ordinal, address)
                address += 1000
        lines += root_record('long-close', 6, 0, 2000)
        self.path = self.directory / 'trace-main'
        self.path.write_text('\n'.join(lines) + '\n')
    def tearDown(self):
        self.temporary.cleanup()
    def test_every_actual_root_and_retained_graph(self):
        result = traces.tree_trace(self.directory, 0, False)
        self.assertEqual(result['actual_iteration_roots'], 80)
        self.assertTrue(result['actual_distinct_nodes_and_leaf_objects'])
    def test_removed_real_leaf_rejected(self):
        self.path.write_text(self.path.read_text().replace('NODE 1007 0 0\n', ''))
        with self.assertRaises(AssertionError):
            traces.tree_trace(self.directory, 0, False)
    def test_replaced_long_lived_root_rejected(self):
        self.path.write_text(self.path.read_text().replace('ROOT long-close 6 0 2000', 'ROOT long-close 6 0 2001'))
        with self.assertRaises(AssertionError):
            traces.tree_trace(self.directory, 0, False)

    def test_concurrently_live_churn_graph_cannot_overlap_retained_graph(self):
        original = self.path.read_text()
        replacement = '\n'.join(root_record('iteration', 4, 0, 2000))
        changed = re.sub(r'ROOT iteration 4 0 .*?ENDROOT', replacement, original, count=1, flags=re.S)
        self.path.write_text(changed)
        with self.assertRaisesRegex(AssertionError, 'overlaps the retained'):
            traces.tree_trace(self.directory, 0, False)

    def test_actual_worker_iteration_labels_must_match_each_interval(self):
        self.path.unlink()
        main = ['MAIN 99', *root_record('stretch', 7, 0, 1000), *root_record('long-open', 6, 0, 2000)]
        address = 1000  # Valid reuse of the already released stretch addresses.
        for depth in (4, 6):
            iterations = 1 << (6 - depth + 4)
            for worker in range(8):
                first = iterations * worker // 8
                last = iterations * (worker + 1) // 8
                lines = [f'RANGE {depth} {first} {last} {depth * 1000 + worker}']
                for ordinal in range(first, last):
                    lines += root_record('iteration', depth, ordinal, address)
                    address += 1000
                    if address == 2000:
                        address += 1000
                (self.directory / f'trace-{depth}-{first}').write_text('\n'.join(lines) + '\n')
                main.append(f'JOIN {depth} {first} {last}')
        main += root_record('long-close', 6, 0, 2000)
        self.path.write_text('\n'.join(main) + '\n')
        self.assertTrue(traces.tree_trace(self.directory, 0, True)['actual_interval_labels_correlated'])
        first = self.directory / 'trace-4-0'
        second = self.directory / 'trace-4-8'
        first.write_text(first.read_text().replace('ROOT iteration 4 0 ', 'ROOT iteration 4 8 '))
        second.write_text(second.read_text().replace('ROOT iteration 4 8 ', 'ROOT iteration 4 0 '))
        with self.assertRaisesRegex(AssertionError, 'correlate'):
            traces.tree_trace(self.directory, 0, True)


class LoweringChecks(unittest.TestCase):
    def test_original_ir_leaf_path_must_allocate_unconditionally(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'original.ll'
            body = '''define ptr @clbg.binarytrees.make(i32 %depth) {
entry:
 %node = call ptr @malloc(i64 16)
 %test = icmp sgt i32 %depth, 0
 br i1 %test, label %children, label %leaf
children:
 %left = call ptr @clbg.binarytrees.make(i32 0)
 %right = call ptr @clbg.binarytrees.make(i32 0)
 ret ptr %node
leaf:
 ret ptr %node
}
'''
            path.write_text(body)
            self.assertTrue(traces.allocation_llvm([path])['real_allocation'])
            path.write_text(body.replace(' %node = call ptr @malloc(i64 16)\n', '').replace('children:\n', 'children:\n %node = call ptr @malloc(i64 16)\n'))
            with self.assertRaisesRegex(AssertionError, 'before any control split'):
                traces.allocation_llvm([path])
    def test_final_arm_leaf_shortcut_before_allocation_rejected(self):
        good = '''_clbg.binarytrees.make:
 mov x19, x0
 bl _malloc
 cbz w19, leaf
 bl _clbg.binarytrees.make
 bl _clbg.binarytrees.make
 ret
_next:
 ret
'''
        self.assertTrue(traces.allocation_arm(good, 'clbg.binarytrees.make')['allocation_before_leaf_path'])
        # Actual saved scored ARM, not a renamed synthetic allocator call.
        saved = traces.Path(__file__).resolve().parents[4] / 'continuation/jobs/c3-tree-arm-inspection-v1/creator-0.arm'
        actual = saved.read_text()
        proof = traces.allocation_arm(actual, 'clbg.binarytrees.make')
        self.assertEqual(proof['allocator_call']['callee'], 'calloc')
        self.assertEqual(proof['allocator_call']['rendering'], 'otool-symbol-stub')
        self.assertEqual(proof['creator_entry_address'], 0x100002300)
        numeric_children = actual.replace('bl\t_clbg.binarytrees.make', 'bl\t0x100002300')
        self.assertEqual(len(traces.allocation_arm(numeric_children, 'clbg.binarytrees.make')['recursive_call_resolution']), 2)
        for replacement in ('0x100010bf4', '0x100010bf4 ; symbol stub for: _calloc_extra',
                            '0x100010bf4 ; misleading comment _calloc',
                            '0x100010bf4 ; symbol stub for: _calloc ; forged'):
            with self.subTest(unresolved_allocator=replacement):
                with self.assertRaisesRegex(AssertionError, 'actual malloc/calloc'):
                    traces.allocation_arm(actual.replace('0x100010bf4 ; symbol stub for: _calloc', replacement), 'clbg.binarytrees.make')
        with self.assertRaisesRegex(AssertionError, 'both child calls'):
            traces.allocation_arm(numeric_children.replace('bl\t0x100002300', 'bl\t0x100002304'), 'clbg.binarytrees.make')
        with self.assertRaisesRegex(AssertionError, 'both child calls'):
            traces.allocation_arm(numeric_children.replace('bl\t0x100002300',
                'bl\t0x100002304 ; symbol stub for: _clbg.binarytrees.make'), 'clbg.binarytrees.make')
        with self.assertRaisesRegex(AssertionError, 'ambiguous'):
            traces.allocation_arm(actual.replace('_clbg.binarytrees.make:', '_clbg.binarytrees.make.extra:'), 'clbg.binarytrees.make')
        for instruction in ('cbz w0, static_leaf', 'cbnz w0, static_leaf',
                            'tbz w0, #0, static_leaf', 'tbnz w0, #0, static_leaf',
                            'b.cs static_leaf', 'b.cc static_leaf', 'b.mi static_leaf',
                            'b.pl static_leaf', 'b.vs static_leaf', 'b.vc static_leaf',
                            'b.hi static_leaf', 'b.ls static_leaf', 'b static_leaf',
                            'br x0', 'blr x0', 'ret', 'retab'):
            bad = good.replace(' mov x19, x0\n', ' ' + instruction + '\n mov x19, x0\n')
            with self.subTest(instruction=instruction):
                with self.assertRaisesRegex(AssertionError, 'splits before allocation'):
                    traces.allocation_arm(bad, 'clbg.binarytrees.make')



if __name__ == '__main__':
    unittest.main()
