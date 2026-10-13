"""Six unexecuted meaningful fixture methods. Synthetic traces never publish."""
from collections import Counter
import copy
from pathlib import Path
import unittest
import instrument_sources
import work_proofs

HERE = Path(__file__).resolve().parent
BASELINE = HERE / 'original-sources'


def actual_graph(depth):
    rows = []
    def visit(remaining):
        identifier = len(rows)+1
        row = dict(id=identifier,left=0,right=0,leaf=(remaining == 0))
        rows.append(row)
        if remaining:
            row['left'] = visit(remaining-1)
            row['right'] = visit(remaining-1)
        return identifier
    root = visit(depth)
    return dict(root=root,nodes=rows,distinct_nodes=len(rows))


def numbered(rows):
    for ordinal,row in enumerate(rows,1):
        row.update(ordinal=ordinal,worker='main')
    return rows


def tree_trace(n):
    maximum = max(6,n)
    stretch = actual_graph(maximum+1)
    retained = actual_graph(maximum)
    rows = [dict(event='stretch_checked',depth=maximum+1,check=len(stretch['nodes']),
                 graph=stretch,retained_across_callback=False),
            dict(event='retain',depth=maximum,graph=retained)]
    for depth in range(4,maximum+1,2):
        for remaining in range(1 << (maximum-depth+4),0,-1):
            live = actual_graph(depth)
            rows += [dict(event='temporary_begin',remaining=remaining,depth=depth),
                     dict(event='temporary_checked',remaining=remaining,depth=depth,
                          check=len(live['nodes']),graph=live,concurrent_retained_disjoint=True)]
    rows.append(dict(event='retained_checked',depth=maximum,check=len(retained['nodes']),graph=retained,
                     actual_root_identity=True,all_actual_node_identities=True))
    return numbered(rows)


def map_trace(sequence):
    rows = []
    for identifier,width in enumerate(work_proofs.WIDTHS,1):
        counts = Counter(work_proofs.encoded(sequence[start:start+width].upper())
                         for start in range(max(0,len(sequence)-width+1)))
        capacity = 0
        if counts:
            capacity = 4
            while capacity < 2*len(counts):
                capacity *= 2
        rows += [dict(event='map_begin',map_id=identifier,width=width,size=0,capacity=0,bucket_arity=1),
                 dict(event='map_completed',map_id=identifier,width=width,sequence=sequence,
                      sequence_length=len(sequence),size=len(counts),capacity=capacity,
                      bucket_arity=2*capacity+1,entries=[dict(key=key,value=value) for key,value in counts.items()],
                      actual_begin_root_identity=True,per_window_trace_recorded=False)]
    rows.append(dict(event='maps_returned',map_ids=list(range(1,8)),
                     actual_all_root_identities=True,single_worker=True,joins_required=False))
    return numbered(rows)


def pi_trace(n):
    q,r,t,k,count = 1,0,1,1,0
    rows = [dict(event='pi_begin',requested=n)]
    while count < n:
        n3,n4 = 3*q+r,4*q+r
        accepted = n3//t == n4//t
        rows.append(dict(event='pi_candidate',requested=n,count=count,q=q,r=r,t=t,k=k,
            numerator3=n3,numerator4=n4,quotient3=n3//t,quotient4=n4//t,accepted=accepted))
        if accepted:
            q,r = 10*q,10*(r-(n3//t)*t)
            count += 1
        else:
            factor = 2*k+1
            q,r,t,k = q*k,(2*q+r)*factor,t*factor,k+1
    rows.append(dict(event='pi_finished',requested=n,observed_accepted=count))
    return numbered(rows)


class SourceHooks(unittest.TestCase):
    def test_exact_reverse_projection_preserves_all_original_source_bytes(self):
        for bench in ('binarytrees','knucleotide','pidigits'):
            for form in ('compact','optimized'):
                source = (BASELINE / 'sources' / bench / (form+'.pl')).read_text()
                observed,edits = instrument_sources.transform(bench,source,HERE/'observer.pl')
                self.assertTrue(edits and 'clbg_observer:' in observed)
                with self.assertRaises(AssertionError):
                    instrument_sources.transform(bench,observed,HERE/'observer.pl')
                before = instrument_sources.HOOKS[bench][0][0]
                with self.assertRaises(AssertionError):
                    instrument_sources.transform(bench,source.replace(before,'MISSING_ACTUAL_SOURCE_HOOK'),HERE/'observer.pl')


class TreeWork(unittest.TestCase):
    def test_complete_source_iteration_ranges_and_retained_graphs(self):
        positive = tree_trace(0)
        result = work_proofs.trees(positive,0)
        self.assertTrue(result['original_check_counts_reconciled'])
        for mutate in (
            lambda trace: trace[3].update(remaining=1),
            lambda trace: trace[3].update(worker='foreign-worker'),
            lambda trace: trace[3].update(concurrent_retained_disjoint=False),
            lambda trace: trace[-1].update(actual_root_identity=False),
            lambda trace: trace[-1].update(graph=actual_graph(5)),
        ):
            changed = copy.deepcopy(positive)
            mutate(changed)
            with self.assertRaises(AssertionError):
                work_proofs.trees(changed,0)

    def test_shared_leaf_or_missing_second_real_child_is_rejected(self):
        positive = actual_graph(2)
        self.assertEqual(work_proofs.graph(positive,2,7),7)
        for mutate in (
            lambda g: g['nodes'][0].update(right=g['nodes'][0]['left']),
            lambda g: g['nodes'][1].update(right=0),
            lambda g: g['nodes'][-1].update(id=1),
            lambda g: g.update(distinct_nodes=False),
        ):
            changed = copy.deepcopy(positive)
            mutate(changed)
            with self.assertRaises(AssertionError):
                work_proofs.graph(changed,2,7)


class MapWork(unittest.TestCase):
    def test_all_seven_actual_maps_capacity_and_overlapping_tail_totals(self):
        sequence = 'ggtattttaatttatagtacgtggt'
        positive = map_trace(sequence)
        self.assertEqual(len(work_proofs.maps(positive,sequence)['maps']),7)
        for mutate in (
            lambda trace: trace[1]['entries'].pop(),
            lambda trace: trace[1].update(capacity=0,bucket_arity=1),
            lambda trace: trace[13].update(sequence_length=False),
            lambda trace: trace[-1].update(map_ids=[1,2,3,4,5,6,6]),
            lambda trace: trace[11].update(actual_begin_root_identity=False),
        ):
            changed = copy.deepcopy(positive)
            mutate(changed)
            with self.assertRaises(AssertionError):
                work_proofs.maps(changed,sequence)

    def test_empty_maps_are_real_distinct_default_maps_and_zero_is_not_false(self):
        positive = map_trace('')
        self.assertEqual(work_proofs.maps(positive,'')['maps'][0]['complete_windows'],0)
        for changes in (dict(size=False),dict(capacity=False),dict(bucket_arity=True)):
            changed = copy.deepcopy(positive)
            changed[0].update(changes)
            with self.assertRaises(AssertionError):
                work_proofs.maps(changed,'')


class PiWork(unittest.TestCase):
    def test_actual_both_divisions_zero_tail_and_independent_golden_digits(self):
        golden = '31415926535897932384626433832'
        for n in (0,1,9,10,11,27):
            positive = pi_trace(n)
            self.assertEqual(work_proofs.pi(positive,n,golden[:n])['observed_digits'],n)
        positive = pi_trace(11)
        for changes in (dict(quotient4=999),dict(numerator3=999),dict(count=False),dict(accepted=True)):
            changed = copy.deepcopy(positive)
            changed[1].update(changes)
            with self.assertRaises(AssertionError):
                work_proofs.pi(changed,11,golden[:11])
        with self.assertRaises(AssertionError):
            work_proofs.pi(copy.deepcopy(positive),11,'00000000000')
