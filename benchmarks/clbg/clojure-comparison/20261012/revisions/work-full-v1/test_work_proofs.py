"""Unexecuted bounded corruption fixtures for actual Clojure observer records."""
import copy
from itertools import product
import unittest

import work_proofs


def tree_record(depth, kind, next_identity, **fields):
    nodes = []
    def make(remaining):
        identity = next(next_identity)
        left, right = (make(remaining-1), make(remaining-1)) if remaining else (None, None)
        nodes.append(dict(id=identity, left=left, right=right, array_length=2))
        return identity
    root = make(depth)
    return dict(kind=kind, depth=depth, root_id=root, nodes=nodes,
                actual_recursive_check=len(nodes), actual_checker_class='clbg.binarytrees.compact$check_tree', thread_id=7, **fields)


def tree_document(variant='compact'):
    from itertools import count
    identities = count(1)
    stretch = tree_record(7, 'stretch', identities)
    retained = tree_record(6, 'retained-before', identities, phase='before-temporary-work')
    after = dict(copy.deepcopy(retained), kind='retained-after', phase='after-all-temporary-work')
    temporary = [tree_record(depth, 'temporary', identities, iteration=rank, worker=0)
                 for depth in (4, 6) for rank in range(1 << (10-depth))]
    if variant == 'optimized':
        for row in (stretch, retained, after, *temporary):
            row['actual_checker_class'] = 'clbg.binarytrees.optimized$check_tree'
        for row in temporary:
            iterations = 1 << (10-row['depth'])
            row['worker'] = row['iteration']*8//iterations
            row['thread_id'] = row['depth']*100+row['worker']+10
    workers = 8 if variant == 'optimized' else 1
    return dict(n=0, variant=variant, graphs=[stretch, retained, *temporary, after],
        retained_root_read_after_all_worker_joins=True,
        interval_events=[dict(depth=depth, worker=worker, begin=(1 << (10-depth))*worker//workers,
            end=(1 << (10-depth))*(worker+1)//workers,
            thread_id=depth*100+worker+10 if variant == 'optimized' else 7, event=event)
            for depth in (4, 6) for worker in range(workers) for event in ('started', 'finished')],
        worker_joins=[dict(all_gets_completed=True, thread_id=7,
            futures=[dict(object_id=depth*1000+worker+1, done=True) for worker in range(workers)],
            returned_numbers=[((1 << (10-depth))//workers)*((1 << (depth+1))-1)]*workers)
            for depth in (4, 6)] if variant == 'optimized' else [],
        actual_total_node_allocations=sum(len(row['nodes']) for row in [stretch, retained, *temporary]))


def map_document():
    sequence = ''.join(''.join(row) for row in product('ACGT', repeat=3))
    maps = []
    for index, width in enumerate(work_proofs.WIDTHS):
        updates, counts = [], {}
        for position in range(max(0, len(sequence)-width+1)):
            word = sequence[position:position+width]
            key = 0
            for char in word:
                key = (key << 2) | 'ACGT'.index(char)
            old = counts.get(key, 0)
            updates.append(dict(position=position, key=key, old=old, new=old+1, worker=index, thread_id=index+10))
            counts[key] = old+1
        maps.append(dict(width=width, sequence=sequence, initial_count=0, final_count=len(counts), private_owner=True, owner=index,
            initial_storage_class='java.util.HashMap', returned_storage_class='java.util.HashMap',
            initial_object_id=index+1, returned_object_id=index+1, received_object_id=index+1,
            thread_id=index+10, returned_thread_id=index+10,
            parent_read_after_worker_join=True, actual_updates=updates,
            actual_entries=[dict(key=key, count=value) for key, value in counts.items()],
            initial_storage_slots=16, peak_storage_slots=512))
    return dict(sequence=sequence, variant='optimized', maps=maps, retained_returned_map_ids=list(range(1, 8)),
        worker_joins=[dict(all_gets_completed=True, returned_object_ids=list(range(1, 8)), thread_id=1,
            futures=[dict(object_id=index+100, done=True) for index in range(7)])])


def pi_document(wanted):
    # Independently transcribed two-digit golden operands. Do not generate a
    # fixture by replaying the validator's recurrence/division implementation.
    assert wanted in (0, 2)
    rows = [
        dict(q=1, r=0, t=1, k=1, emitted=0, numerator_three=3, numerator_four=4,
             divisor_three=1, divisor_four=1, quotient_three=3, quotient_four=4, accepted=False),
        dict(q=1, r=6, t=3, k=2, emitted=0, numerator_three=9, numerator_four=10,
             divisor_three=3, divisor_four=3, quotient_three=3, quotient_four=3, accepted=True),
        dict(q=10, r=-30, t=3, k=2, emitted=1, numerator_three=0, numerator_four=10,
             divisor_three=3, divisor_four=3, quotient_three=0, quotient_four=3, accepted=False),
        dict(q=20, r=-50, t=15, k=3, emitted=1, numerator_three=10, numerator_four=30,
             divisor_three=15, divisor_four=15, quotient_three=0, quotient_four=2, accepted=False),
        dict(q=60, r=-70, t=105, k=4, emitted=1, numerator_three=110, numerator_four=170,
             divisor_three=105, divisor_four=105, quotient_three=1, quotient_four=1, accepted=True),
    ]
    return dict(wanted=wanted, extractions=rows if wanted else [], emitted_digits='31' if wanted else '')


def tree_events(document):
    graphs = document['graphs']
    events = [dict(kind='tree', value=row) for row in graphs[:2]]
    joins = iter(document['worker_joins'])
    intervals = document['interval_events']
    for index in range(0, len(intervals), 2):
        start, end = intervals[index:index+2]
        events.append(dict(kind='interval', value=start))
        events.extend(dict(kind='tree', value=row) for row in graphs if row['kind'] == 'temporary' and
            row['depth'] == start['depth'] and row['worker'] == start['worker'])
        events.append(dict(kind='interval', value=end))
        if document['variant'] == 'optimized' and (index+2 == len(intervals) or intervals[index+2]['depth'] != start['depth']):
            events.append(dict(kind='joins', value=next(joins)))
    events.extend((dict(kind='tree', value=graphs[-1]), dict(kind='document', value=document)))
    return events


def map_events(document):
    events = []
    for row in document['maps']:
        events.append(dict(kind='map-start', value=row))
        events.extend(dict(kind='map-write', value=dict(update, width=row['width'],
            object_id=row['initial_object_id'], storage_slots=row['peak_storage_slots'])) for update in row['actual_updates'])
        events.append(dict(kind='map-return', value=row))
    events.extend(dict(kind='joins', value=join) for join in document['worker_joins'])
    events.extend((dict(kind='maps-received', value=dict(object_ids=document['retained_returned_map_ids'], thread_id=1)),
                   dict(kind='document', value=document)))
    return events


def mutex_document():
    return dict(native_pid=123, native_pgid=123, inherited_fd=64,
        authoritative_native_identity=dict(pid=123, pgid=123, inherited_common_mutex=True,
            command=['java', '-cp', 'classes:jars', 'namespace', '9']), native_command=['java', '-cp', 'classes:jars', 'namespace', '9'],
        reviewed_command=['java', '-cp', 'classes:jars', 'namespace', '9'], observer_owned_group=True,
        observer_exit_status=0, observer_reaped=True, observer_remaining_group=[], observer_cleanup_errors=[],
        observer_signalled_foreign_group=False, supervisor_common_device_inode=[1, 88],
        actual_native_open_files=[dict(pid=123, fd=64, path='/tmp/mech-clbg-timing.lock', device_inode=[1, 88])],
        actual_inherited_description_verified=True)


class WorkProofChecks(unittest.TestCase):
    def test_tree_requires_actual_fresh_leaves_and_distinct_graph_storage(self):
        value = tree_document()
        self.assertTrue(all(work_proofs.trees(value, 0, 'compact').values()))
        broken = copy.deepcopy(value)
        del broken['graphs'][0]['nodes'][0]
        with self.assertRaises(AssertionError):
            work_proofs.trees(broken, 0, 'compact')
        for mutation in ('missing-interval', 'wrong-owner', 'wrong-thread', 'unfinished-interval', 'copied-checker'):
            broken = copy.deepcopy(value)
            if mutation == 'missing-interval':
                broken['interval_events'].pop()
            elif mutation == 'wrong-owner':
                broken['graphs'][2]['worker'] = 1
            elif mutation == 'wrong-thread':
                broken['graphs'][2]['thread_id'] = 999
            elif mutation == 'unfinished-interval':
                broken['interval_events'][1]['event'] = 'started'
            else:
                broken['graphs'][2]['actual_checker_class'] = 'clbg.binarytreesobserver.compact$check_tree'
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.trees(broken, 0, 'compact')
        optimized = tree_document('optimized')
        self.assertTrue(all(work_proofs.trees(optimized, 0, 'optimized').values()))
        for mutation in ('missing-join', 'unfinished-future', 'wrong-returned-total'):
            broken = copy.deepcopy(optimized)
            if mutation == 'missing-join':
                broken['worker_joins'].pop()
            elif mutation == 'unfinished-future':
                broken['worker_joins'][0]['futures'][0]['done'] = False
            else:
                broken['worker_joins'][0]['returned_numbers'][0] += 1
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.trees(broken, 0, 'optimized')
        broken = copy.deepcopy(value)
        parent = next(node for node in broken['graphs'][0]['nodes'] if node['left'] is not None)
        parent['right'] = parent['left']
        with self.assertRaises(AssertionError):
            work_proofs.trees(broken, 0, 'compact')

    def test_retained_graph_identity_must_survive_actual_churn(self):
        value = tree_document()
        work_proofs.trees(value, 0, 'compact')
        broken = copy.deepcopy(value)
        broken['graphs'][-1]['root_id'] = broken['graphs'][0]['root_id']
        with self.assertRaises(AssertionError):
            work_proofs.trees(broken, 0, 'compact')
        for variant in ('compact', 'optimized'):
            value = tree_document(variant)
            events = tree_events(value)
            self.assertTrue(work_proofs.event_order(events, value, 'binarytrees', variant))
            for mutation in ('finished-before-root', 'retained-before-after-start', 'join-before-finish'):
                if mutation == 'join-before-finish' and variant == 'compact':
                    continue
                broken = copy.deepcopy(events)
                root = next(i for i, row in enumerate(broken) if row['kind'] == 'tree' and row['value']['kind'] == 'temporary')
                finished = next(i for i, row in enumerate(broken) if row['kind'] == 'interval' and row['value']['event'] == 'finished')
                if mutation == 'finished-before-root':
                    broken[root], broken[finished] = broken[finished], broken[root]
                elif mutation == 'retained-before-after-start':
                    started = next(i for i, row in enumerate(broken) if row['kind'] == 'interval')
                    broken[1], broken[started] = broken[started], broken[1]
                else:
                    joined = next(i for i, row in enumerate(broken) if row['kind'] == 'joins')
                    broken[joined], broken[finished] = broken[finished], broken[joined]
                # Keep the final aggregate faithful to the altered raw order;
                # rejection must come from chronology, not a vector mismatch.
                aggregate = broken[-1]['value']
                for kind, key in (('tree', 'graphs'), ('interval', 'interval_events'), ('joins', 'worker_joins')):
                    aggregate[key] = [row['value'] for row in broken if row['kind'] == kind]
                with self.subTest(variant=variant, mutation=mutation), self.assertRaises(AssertionError):
                    work_proofs.event_order(broken, aggregate, 'binarytrees', variant)
        broken = copy.deepcopy(value)
        broken['graphs'][2] = dict(copy.deepcopy(broken['graphs'][1]), kind='temporary', depth=4, iteration=0, worker=0)
        with self.assertRaises(AssertionError):
            work_proofs.trees(broken, 0, 'compact')

    def test_all_seven_growing_map_inventories_and_handoff_are_required(self):
        value = map_document()
        self.assertTrue(all(work_proofs.maps(value, value['sequence'], 'optimized', True).values()))
        for mutation in ('omitted-map', 'lost-entry', 'foreign-handoff', 'no-growth', 'wrong-worker', 'wrong-thread', 'unjoined-worker'):
            broken = copy.deepcopy(value)
            if mutation == 'omitted-map':
                broken['maps'].pop()
            elif mutation == 'lost-entry':
                broken['maps'][2]['actual_entries'].pop()
            elif mutation == 'foreign-handoff':
                broken['maps'][2]['received_object_id'] = 999
            elif mutation == 'no-growth':
                for row in broken['maps']:
                    row['peak_storage_slots'] = row['initial_storage_slots']
            elif mutation == 'wrong-worker':
                broken['maps'][2]['actual_updates'][0]['worker'] = 7
            elif mutation == 'wrong-thread':
                broken['maps'][2]['returned_thread_id'] = 999
            else:
                broken['worker_joins'][0]['futures'][0]['done'] = False
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.maps(broken, value['sequence'], 'optimized', True)

    def test_every_overlapping_window_including_tail_is_required(self):
        value = map_document()
        work_proofs.maps(value, value['sequence'], 'optimized', True)
        for index in (0, -1):
            broken = copy.deepcopy(value)
            del broken['maps'][-1]['actual_updates'][index]
            with self.subTest(index=index), self.assertRaises(AssertionError):
                work_proofs.maps(broken, value['sequence'], 'optimized', True)
        broken = copy.deepcopy(value)
        broken['maps'][1]['actual_updates'][-1]['new'] += 1
        with self.assertRaises(AssertionError):
            work_proofs.maps(broken, value['sequence'], 'optimized', True)
        events = map_events(value)
        self.assertTrue(work_proofs.event_order(events, value, 'knucleotide', 'optimized'))
        for mutation in ('return-before-last-write', 'receive-before-join'):
            broken = copy.deepcopy(events)
            if mutation == 'return-before-last-write':
                returned = next(i for i, row in enumerate(broken) if row['kind'] == 'map-return')
                broken[returned], broken[returned-1] = broken[returned-1], broken[returned]
            else:
                joined = next(i for i, row in enumerate(broken) if row['kind'] == 'joins')
                received = next(i for i, row in enumerate(broken) if row['kind'] == 'maps-received')
                broken[joined], broken[received] = broken[received], broken[joined]
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.event_order(broken, broken[-1]['value'], 'knucleotide', 'optimized')

    def test_two_actual_pi_divisions_recurrence_and_zero_extraction_are_required(self):
        value = pi_document(2)
        self.assertTrue(work_proofs.pi(value, 2)['accepted_and_rejected_recurrence'])
        self.assertTrue(work_proofs.pi(pi_document(0), 0)['zero_digits_no_extraction'])
        for mutation in ('copied-quotient', 'altered-operand', 'omitted-candidate', 'zero-extraction'):
            broken = copy.deepcopy(value)
            wanted = 2
            if mutation == 'copied-quotient':
                broken['extractions'][0]['quotient_four'] = broken['extractions'][0]['quotient_three']
            elif mutation == 'altered-operand':
                broken['extractions'][1]['numerator_four'] += 1
            elif mutation == 'omitted-candidate':
                del broken['extractions'][1]
            else:
                broken, wanted = dict(pi_document(0), extractions=[value['extractions'][0]]), 0
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.pi(broken, wanted)

    def test_native_mutex_description_and_owned_observer_cleanup_are_required(self):
        value = mutex_document()
        self.assertTrue(all(work_proofs.native_mutex(value).values()))
        for mutation in ('foreign-pid', 'foreign-file', 'missing-description', 'cleanup-failure', 'remaining-group'):
            broken = copy.deepcopy(value)
            if mutation == 'foreign-pid':
                broken['actual_native_open_files'][0]['pid'] = 999
            elif mutation == 'foreign-file':
                broken['actual_native_open_files'][0]['device_inode'] = [1, 99]
            elif mutation == 'missing-description':
                broken['actual_inherited_description_verified'] = False
            elif mutation == 'cleanup-failure':
                broken['observer_cleanup_errors'] = ['fixture failed cleanup']
            else:
                broken['observer_remaining_group'] = [111]
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                work_proofs.native_mutex(broken)
