"""Meaningful observer-corruption fixtures; never execute native programs."""
from collections import Counter
from pathlib import Path
import tempfile
import re
import unittest

import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "private-v1"))
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
            records = [f'OWNER {width} {index + 100}', f'MAP {width} {len(counts)} 16 99']
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
    GOOD = '''{beam_file,benchmark,[],[],[],[
{function,make,1,12,[
 {call_ext,0,{extfunc,clbg_reference,new,0}},
 {put_tuple2,{x,0},{list,[{atom,leaf},{x,0}]}},
 {call,1,{f,12}}, {call,1,{f,12}},
 {put_tuple2,{x,0},{list,[{atom,node},{y,0},{x,0},{y,1}]}}
]},
{function,check,1,13,[{return}]},
{function,depths,2,14,[{return}]},
{function,main,0,15,[
 {call,1,{f,12}}, {call,1,{f,13}},
 {call,1,{f,12}}, {move,{x,0},{y,4}},
 {call,2,{f,14}}, {move,{y,4},{x,0}}, {call,1,{f,13}}
]}
]}'''
    REFERENCE = '{function,new,0,1,[{call_ext_only,0,{extfunc,erlang,make_ref,0}}]}'

    def test_actual_scored_beam_dynamic_leaf_and_retention(self):
        result = traces.allocation_beam(self.GOOD, self.REFERENCE)
        self.assertTrue(result['dynamic_leaf_and_interior_tuples'])
        self.assertTrue(result['retained_scored_long_lived_root'])
        inlined = self.GOOD.replace('{extfunc,clbg_reference,new,0}', '{extfunc,erlang,make_ref,0}')
        self.assertTrue(traces.allocation_beam(inlined, self.REFERENCE)['fresh_reference_identity'])

    def test_constant_leaf_or_lost_retained_root_rejected(self):
        for alteration in ('constant-leaf', 'missing-reference', 'one-child', 'different-stack-root', 'overwritten-stack-root'):
            text, reference = self.GOOD, self.REFERENCE
            if alteration == 'constant-leaf': text = text.replace('{put_tuple2,{x,0},{list,[{atom,leaf},{x,0}]}}', '{move,{literal,{leaf,constant}},{x,0}}')
            elif alteration == 'missing-reference': reference = reference.replace('make_ref','cached_reference')
            elif alteration == 'one-child': text = text.replace('{call,1,{f,12}}, {call,1,{f,12}},', '{call,1,{f,12}},')
            elif alteration == 'overwritten-stack-root': text = text.replace('{call,2,{f,14}},', '{call,2,{f,14}}, {move,{atom,nil},{y,4}},')
            else: text = text.replace('{move,{y,4},{x,0}}','{move,{y,3},{x,0}}')
            with self.subTest(alteration=alteration), self.assertRaises(AssertionError):
                traces.allocation_beam(text, reference)


if __name__ == '__main__':
    unittest.main(verbosity=2)

