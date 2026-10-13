"""Draft pure Python observer fixtures; no native process or test run on import."""
from pathlib import Path
import fcntl
import tempfile
import unittest

import binarytrees_trace as proof
import full_study

HERE = Path(__file__).resolve().parent
FD = None


class TreeTraceChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Root must execute these fixtures under the common timing mutex')
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.address = 1000
        self.graphs = {}
        self.maximum = 6
        self.stdout = proof._expected_stdout(self.maximum)
        self.make_root('stretch', 7, 0)
        before = self.make_root('long-before', 6, 0)
        self.make_root('long-after', 6, 0, graph=before)
        for depth in (4, 6):
            count = 1 << (self.maximum-depth+4)
            nodes = (1 << (depth+1))-1
            for index in range(count):
                self.make_root('temporary', depth, index, cumulative=(index+1)*nodes)

    def graph(self, depth):
        rows = {}

        def tree(level):
            self.address += 8
            address = self.address
            left = tree(level-1) if level else 0
            right = tree(level-1) if level else 0
            rows[address] = (left, right, level)
            return address

        root = tree(depth)
        return root, rows

    def save_root(self, kind, depth, index, root, rows, cumulative):
        path = self.directory/f'{kind}-{depth}-{index}.trace'
        lines = [f'TRACE_TREE_V1|{kind}|{depth}|{index}|{root}|{cumulative}']
        lines += [f'NODE|{address}|{left}|{right}|{level}' for address,(left,right,level) in rows.items()]
        lines.append(f'END_TREE_V1|{len(rows)}|{len(rows)}')
        path.write_text('\n'.join(lines)+'\n')
        self.graphs[kind,depth,index] = (root,rows,cumulative)
        return path

    def make_root(self, kind, depth, index, cumulative=0, graph=None):
        root,rows = graph or self.graph(depth)
        self.save_root(kind,depth,index,root,rows,cumulative)
        return root,rows

    def reconcile(self, variant='compact'):
        return proof.reconcile(self.directory,self.stdout,6,variant,FD)

    def test_complete_actual_root_graph_fixture_is_accepted_for_both_forms(self):
        for variant in ('compact','optimized'):
            with self.subTest(variant=variant):
                result = self.reconcile(variant)
                self.assertEqual(result['temporary_roots'],80)
                self.assertEqual(result['actual_roots'],83)
                self.assertTrue(result['retained_root_and_all_edges_identical'])
                self.assertTrue(result['distinct_nodes_and_leaves_per_root'])
                self.assertEqual(result['scored_samples'],0)

    def test_observer_source_keeps_node_creator_check_and_stdout(self):
        for variant in ('compact','optimized'):
            with self.subTest(variant=variant):
                source = (HERE/f'revisions/input-v2/sources/binarytrees/{variant}.v').read_text()
                observed = proof.instrument(source,variant)
                self.assertEqual(proof._function(source,'create'),proof._function(observed,'create'))
                self.assertEqual(proof._function(source,'check'),proof._function(observed,'check'))
                self.assertEqual(source.count('&Node{'),observed.count('&Node{'))
                self.assertEqual(source.count('sum += check(tree)'),observed.count('sum += check(tree)'))
                self.assertEqual(source.count('println('),observed.count('println('))
                self.assertNotIn('audit_tree',source)

    def test_shared_actual_leaf_is_rejected(self):
        key = ('temporary',4,0)
        root,original,cumulative = self.graphs[key]
        rows = dict(original)
        parent = next(address for address,row in rows.items() if row[2] == 1)
        left,right,level = rows[parent]
        rows[parent] = (left,left,level)
        self.save_root(*key,root,rows,cumulative)
        with self.assertRaises(AssertionError):
            self.reconcile()

    def test_missing_prescribed_root_is_rejected(self):
        (self.directory/'temporary-4-63.trace').unlink()
        with self.assertRaises(AssertionError):
            self.reconcile()

    def test_changed_retained_whole_graph_is_rejected(self):
        self.make_root('long-after',6,0)
        with self.assertRaises(AssertionError):
            self.reconcile()

    def test_wrong_actual_iteration_cumulative_check_is_rejected(self):
        key = ('temporary',6,15)
        root,rows,cumulative = self.graphs[key]
        self.save_root(*key,root,rows,cumulative+1)
        with self.assertRaises(AssertionError):
            self.reconcile()

    def test_incomplete_original_stdout_is_rejected(self):
        self.stdout = self.stdout[:-1]
        with self.assertRaises(AssertionError):
            self.reconcile()

    def test_wrong_or_unheld_mutex_descriptor_is_rejected(self):
        with self.assertRaises(AssertionError):
            proof.reconcile(self.directory,self.stdout,6,'compact',None)
        with (self.directory/'different-lock').open('w+') as unrelated:
            with self.assertRaises(AssertionError):
                proof.reconcile(self.directory,self.stdout,6,'compact',unrelated.fileno())
        with Path('/tmp/mech-clbg-timing.lock').open('a+') as unheld:
            with self.assertRaises((AssertionError,BlockingIOError)):
                proof.reconcile(self.directory,self.stdout,6,'compact',unheld.fileno())


def main():
    global FD
    module = full_study.configure('binarytrees')
    print('WAIT common mutex: V tree observer counterexample fixtures',flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        FD = lock.fileno()
        destination = HERE/'binarytrees-trace-fixtures.json'
        assert not destination.exists(),'Never overwrite a fixture attempt'
        files = [Path(__file__).resolve(),HERE/'binarytrees_trace.py',
                 HERE/'qualify_binarytrees.py',HERE/'BINARYTREES_TRACE_PLAN.md',
                 HERE/'source_trace.py',HERE/'full_study.py']
        files += [HERE/f'revisions/input-v2/sources/binarytrees/{variant}.v'
                  for variant in ('compact','optimized')]
        pins = {str(path):module.sha(path) for path in files}
        result = unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(TreeTraceChecks))
        module.load_harness().PinGuard(pins).verify()
        module.save(destination,dict(status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
            created_at=module.now(),pins=pins,pins_unchanged_at_end=True,
            native_probes=0,full_performance_runs=0,never_chart_eligible=True))
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
