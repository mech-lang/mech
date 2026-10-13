"""Unexecuted pure validators for actual observer records, never timing data.

Caller supplies original input/size and retained native/observer records. These
functions do not launch, import benchmark code, read files, hash or admit rows.
An authored observer and exact-source edit/bytecode review remain prerequisites.
"""
from collections import Counter


def integer(value, minimum=0):
    assert type(value) is int and value >= minimum
    return value


def graph(record, depth):
    nodes = record['nodes']
    assert nodes and len(nodes) == len({node['id'] for node in nodes})
    indexed = {integer(node['id'], 1): node for node in nodes}
    visited = set()
    def visit(identity, remaining):
        assert identity in indexed and identity not in visited, 'Missing, cyclic or shared actual tree node'
        visited.add(identity)
        node = indexed[identity]
        assert node['array_length'] == 2
        if remaining:
            assert node['left'] is not None and node['right'] is not None
            visit(integer(node['left'], 1), remaining-1)
            visit(integer(node['right'], 1), remaining-1)
        else:
            assert node['left'] is None and node['right'] is None, 'Actual fresh leaf storage is required'
    visit(integer(record['root_id'], 1), depth)
    assert visited == set(indexed), 'Every retained graph record must be reachable actual storage'
    assert len(visited) == (1 << (depth + 1)) - 1
    assert record['actual_recursive_check'] == len(visited)
    return indexed


def trees(document, n, variant):
    maximum = max(6, integer(n))
    assert variant in {'compact', 'optimized'}
    assert document['n'] == n and document['variant'] == variant
    records = document['graphs']
    assert all(record['actual_checker_class'] == 'clbg.binarytrees.' + variant + '$check_tree' for record in records), 'Each graph must use the actual scored compiled checker'
    stretch = [item for item in records if item['kind'] == 'stretch']
    before = [item for item in records if item['kind'] == 'retained-before']
    after = [item for item in records if item['kind'] == 'retained-after']
    temporary = [item for item in records if item['kind'] == 'temporary']
    assert len(records) == len(stretch) + len(before) + len(after) + len(temporary)
    assert len(stretch) == len(before) == len(after) == 1
    seen = set(graph(stretch[0], maximum+1))
    retained = graph(before[0], maximum)
    assert not seen & set(retained)
    seen |= set(retained)
    assert graph(after[0], maximum) == retained and after[0]['root_id'] == before[0]['root_id']
    assert before[0]['phase'] == 'before-temporary-work' and after[0]['phase'] == 'after-all-temporary-work'
    intervals = {}
    events = document['interval_events']
    expected_intervals = {(depth, worker, (iterations*worker)//workers, (iterations*(worker+1))//workers)
        for depth in range(4, maximum+1, 2)
        for iterations in [1 << (maximum-depth+4)]
        for workers in [1 if variant == 'compact' else min(8, iterations)] for worker in range(workers)}
    assert len(events) == len(expected_intervals)*2
    for event in events:
        key = (event['depth'], event['worker'], event['begin'], event['end'])
        assert key in expected_intervals and event['event'] in {'started', 'finished'}
        pair = intervals.setdefault(key, {})
        assert event['event'] not in pair
        pair[event['event']] = integer(event['thread_id'], 1)
    assert set(intervals) == expected_intervals
    assert all(set(pair) == {'started', 'finished'} and pair['started'] == pair['finished'] for pair in intervals.values())
    observed = set()
    expected = {(depth, rank) for depth in range(4, maximum+1, 2)
                for rank in range(1 << (maximum-depth+4))}
    for item in temporary:
        depth, rank = integer(item['depth'], 4), integer(item['iteration'])
        key = (depth, rank)
        assert key in expected and key not in observed
        observed.add(key)
        worker = integer(item['worker'])
        assert worker < (1 if variant == 'compact' else 8)
        owner = [pair for key, pair in intervals.items() if key[0] == depth and key[1] == worker and key[2] <= rank < key[3]]
        assert len(owner) == 1 and item['thread_id'] == owner[0]['started'], 'Real root must belong to its observed interval/thread'
        current = graph(item, depth)
        assert not seen & set(current), 'Fresh temporary graph cannot share storage with prior or retained graphs'
        seen |= set(current)
    assert observed == expected
    joins = document['worker_joins']
    depths = list(range(4, maximum+1, 2))
    assert len(joins) == (len(depths) if variant == 'optimized' else 0)
    for depth, joined in zip(depths, joins):
        workers = min(8, 1 << (maximum-depth+4))
        futures = joined['futures']
        assert joined['all_gets_completed'] is True and len(futures) == len({item['object_id'] for item in futures}) == workers
        assert all(item['done'] is True for item in futures)
        assert joined['returned_numbers'] == [
            ((1 << (maximum-depth+4))*(worker+1)//workers - (1 << (maximum-depth+4))*worker//workers) * ((1 << (depth+1))-1)
            for worker in range(workers)]
        assert integer(joined['thread_id'], 1) == after[0]['thread_id']
    assert document['retained_root_read_after_all_worker_joins'] is True
    assert document['actual_total_node_allocations'] == len(seen)
    return dict(actual_fresh_node_storage=True, actual_leaf_allocation=True,
        retained_identity_after_churn=True, all_iteration_graphs_verified=True, worker_cap_verified=True)


WIDTHS = (1, 2, 3, 4, 6, 12, 18)


def key_text(value, width, variant):
    if variant == 'compact':
        assert type(value) is str and len(value) == width and set(value) <= set('ACGT')
        return value
    integer(value)
    assert value < (1 << (2 * width))
    return ''.join('ACGT'[(value >> (2 * offset)) & 3] for offset in reversed(range(width)))


def maps(document, sequence, variant, require_growth=False):
    assert variant in {'compact', 'optimized'} and set(sequence) <= set('ACGT')
    assert document['sequence'] == sequence and document['variant'] == variant
    observed = document['maps']
    assert len(observed) == len({row['width'] for row in observed}) == 7
    assert {row['width'] for row in observed} == set(WIDTHS)
    assert len({row['initial_object_id'] for row in observed}) == 7
    assert len({row['returned_object_id'] for row in observed}) == 7
    returned, grew = [], False
    for row in observed:
        width = row['width']
        assert row['initial_count'] == 0 and row['private_owner'] is True
        assert row['owner'] == (0 if variant == 'compact' else WIDTHS.index(width))
        thread = integer(row['thread_id'], 1)
        assert row['returned_thread_id'] == thread
        if variant == 'compact':
            assert row['initial_storage_class'] == 'clojure.lang.PersistentHashMap$TransientHashMap'
            assert row['returned_storage_class'] == 'clojure.lang.PersistentHashMap'
        else:
            assert row['initial_storage_class'] == row['returned_storage_class'] == 'java.util.HashMap'
            assert row['initial_object_id'] == row['returned_object_id']
        returned.append(row['returned_object_id'])
        assert row['received_object_id'] == row['returned_object_id']
        assert row['parent_read_after_worker_join'] is True
        windows = max(0, len(sequence)-width+1)
        updates = row['actual_updates']
        assert len(updates) == windows
        current = Counter()
        for position, update in enumerate(updates):
            assert update['position'] == position
            assert update['worker'] == row['owner'] and update['thread_id'] == thread
            word = key_text(update['key'], width, variant)
            assert word == sequence[position:position+width]
            assert type(update['old']) is int and update['old'] == current[word]
            assert type(update['new']) is int and update['new'] == current[word]+1
            current[word] = update['new']
        expected = Counter(sequence[position:position+width] for position in range(windows))
        inventory = row['actual_entries']
        keys = [key_text(item['key'], width, variant) for item in inventory]
        assert len(keys) == len(set(keys))
        final = {key: integer(item['count'], 1) for key, item in zip(keys, inventory)}
        assert final == current == expected
        assert row['final_count'] == len(final)
        initial, peak = integer(row['initial_storage_slots']), integer(row['peak_storage_slots'])
        assert peak >= initial
        # Width1 cannot have more than four possible DNA keys. Require real
        # structural growth beyond a small initial allocation on a richer map,
        # while verifying all seven actual maps/windows in every case.
        grew |= width > 1 and len(final) > 16 and peak > max(16, initial)
    assert document['retained_returned_map_ids'] == returned
    joins = document['worker_joins']
    assert len(joins) == (1 if variant == 'optimized' else 0)
    if joins:
        joined = joins[0]
        assert joined['all_gets_completed'] is True and joined['returned_object_ids'] == returned
        assert len(joined['futures']) == len({row['object_id'] for row in joined['futures']}) == 7
        assert all(row['done'] is True for row in joined['futures'])
        assert integer(joined['thread_id'], 1) not in {row['thread_id'] for row in observed}
    if require_growth:
        assert grew, 'The designated arbitrary DNA growth case needs actual storage growth and complete preserved keys'
    return dict(all_seven_actual_maps=True, all_windows_and_complete_inventories=True,
        actual_growth_observed=grew, private_workers_and_same_map_handoff=True)


def trunc(numerator, denominator):
    assert denominator != 0
    sign = -1 if (numerator < 0) != (denominator < 0) else 1
    return sign * (abs(numerator) // abs(denominator))


def pi(document, wanted):
    integer(wanted)
    assert document['wanted'] == wanted
    q, r, t, k, emitted = 1, 0, 1, 1, 0
    digits, accepted, rejected = [], 0, 0
    for step in document['extractions']:
        assert emitted < wanted, 'No extraction after the complete requested digit prefix'
        assert (step['q'], step['r'], step['t'], step['k'], step['emitted']) == (q, r, t, k, emitted)
        numerator_three, numerator_four = 3*q+r, 4*q+r
        assert (step['numerator_three'], step['numerator_four'], step['divisor_three'], step['divisor_four']) == (numerator_three, numerator_four, t, t)
        three, four = trunc(numerator_three, t), trunc(numerator_four, t)
        assert (step['quotient_three'], step['quotient_four']) == (three, four)
        assert type(step['accepted']) is bool and step['accepted'] is (three == four)
        if step['accepted']:
            assert 0 <= three <= 9
            digits.append(str(three))
            emitted += 1
            accepted += 1
            q, r = q*10, (r-t*three)*10
        else:
            rejected += 1
            factor = 2*k+1
            q, r, t, k = q*k, (r+2*q)*factor, t*factor, k+1
    assert emitted == wanted and document['emitted_digits'] == ''.join(digits)
    if wanted:
        assert accepted and rejected, 'Bounded97 proof must contain real accepted and rejected extraction paths'
    else:
        assert not document['extractions'] and not digits
    return dict(both_actual_extraction_divisions=True, accepted_and_rejected_recurrence=bool(wanted),
                zero_digits_no_extraction=wanted == 0)


def native_mutex(document):
    native_pid, native_pgid = integer(document['native_pid'], 1), integer(document['native_pgid'], 1)
    assert native_pid == native_pgid, 'Exact direct JVM is the authoritative new process-group leader'
    fd = integer(document['inherited_fd'], 3)
    assert document['authoritative_native_identity']['pid'] == native_pid
    assert document['authoritative_native_identity']['pgid'] == native_pgid
    assert document['authoritative_native_identity']['command'] == document['reviewed_command']
    assert document['authoritative_native_identity']['inherited_common_mutex'] is True
    assert document['native_command'] == document['reviewed_command']
    assert document['observer_owned_group'] is True and document['observer_exit_status'] == 0
    assert document['observer_reaped'] is True and document['observer_remaining_group'] == []
    assert document['observer_cleanup_errors'] == [] and document['observer_signalled_foreign_group'] is False
    files = document['actual_native_open_files']
    matching = [row for row in files if row['pid'] == native_pid and row['fd'] == fd and
        row['path'] in ('/tmp/mech-clbg-timing.lock', '/private/tmp/mech-clbg-timing.lock') and row['device_inode'] == document['supervisor_common_device_inode']]
    assert len(matching) == 1
    assert document['actual_inherited_description_verified'] is True
    return dict(same_common_open_file_description=True, bounded_owned_observer_cleanup=True)


def event_order(events, document, benchmark, variant):
    """Reconcile existing ordered raw JSONL evidence, never inferred timestamps."""
    assert events and events[-1] == dict(kind='document', value=document)
    assert sum(row['kind'] == 'document' for row in events) == 1
    if benchmark == 'binarytrees':
        assert all(row['kind'] in {'tree', 'interval', 'joins', 'document'} for row in events)
        stretch = [i for i, row in enumerate(events) if row['kind'] == 'tree' and row['value']['kind'] == 'stretch']
        retained_before = [i for i, row in enumerate(events) if row['kind'] == 'tree' and row['value']['kind'] == 'retained-before']
        retained_after = [i for i, row in enumerate(events) if row['kind'] == 'tree' and row['value']['kind'] == 'retained-after']
        assert len(retained_before) == len(retained_after) == 1
        assert len(stretch) == 1 and stretch[0] < retained_before[0]
        starts, ends, joins = {}, {}, []
        for index, row in enumerate(events):
            value = row['value']
            if row['kind'] == 'interval':
                key = (value['depth'], value['worker'], value['begin'], value['end'], value['thread_id'])
                target = starts if value['event'] == 'started' else ends
                assert key not in target
                target[key] = index
            elif row['kind'] == 'joins':
                joins.append(index)
            elif row['kind'] == 'tree' and value['kind'] == 'temporary':
                candidates = [key for key, start in starts.items() if key[0] == value['depth'] and key[1] == value['worker'] and
                    key[2] <= value['iteration'] < key[3] and key[4] == value['thread_id'] and start < index and key not in ends]
                assert len(candidates) == 1 and retained_before[0] < index < retained_after[0]
        assert starts.keys() == ends.keys() and all(retained_before[0] < starts[key] < ends[key] < retained_after[0] for key in starts)
        if variant == 'optimized':
            depths = sorted({key[0] for key in starts})
            assert len(joins) == len(depths)
            for depth, joined in zip(depths, joins):
                assert all(ends[key] < joined for key in ends if key[0] == depth)
                assert joined < retained_after[0]
                assert all(joined < starts[key] for key in starts if key[0] > depth)
        else:
            assert not joins
            assert all(ends[previous] < starts[current] for previous in ends for current in starts if previous[0] < current[0])
        assert [row['value'] for row in events if row['kind'] == 'tree'] == document['graphs']
        assert [row['value'] for row in events if row['kind'] == 'interval'] == document['interval_events']
        assert [row['value'] for row in events if row['kind'] == 'joins'] == document['worker_joins']
    elif benchmark == 'knucleotide':
        assert all(row['kind'] in {'map-start', 'map-write', 'map-return', 'joins', 'maps-received', 'document'} for row in events)
        starts, ends, updates, joins, received, peak = {}, {}, {}, [], [], {}
        maps_by_width = {row['width']: row for row in document['maps']}
        for index, row in enumerate(events):
            value = row['value']
            if row['kind'] == 'map-start':
                width = value['width']
                assert width not in starts
                starts[width], updates[width] = index, []
                actual = maps_by_width[width]
                for key in ('width', 'sequence', 'initial_object_id', 'initial_storage_class', 'initial_count', 'initial_storage_slots', 'owner', 'thread_id', 'private_owner'):
                    assert value[key] == actual[key]
                peak[width] = value['initial_storage_slots']
            elif row['kind'] == 'map-write':
                width = value['width']
                assert width in starts and width not in ends
                assert value['object_id'] == maps_by_width[width]['initial_object_id']
                peak[width] = max(peak[width], integer(value['storage_slots']))
                updates[width].append({key: value[key] for key in ('position', 'key', 'old', 'new', 'worker', 'thread_id')})
            elif row['kind'] == 'map-return':
                width = value['width']
                assert width in starts and width not in ends
                ends[width] = index
                for key in ('returned_object_id', 'returned_storage_class', 'returned_thread_id', 'final_count', 'actual_entries'):
                    assert value[key] == maps_by_width[width][key]
                peak[width] = max(peak[width], integer(value['peak_storage_slots']))
            elif row['kind'] == 'joins':
                joins.append(index)
                assert set(ends) == set(WIDTHS)
            elif row['kind'] == 'maps-received':
                received.append(index)
                assert set(ends) == set(WIDTHS)
                assert value['object_ids'] == document['retained_returned_map_ids']
                if variant == 'optimized':
                    assert len(joins) == 1 and joins[0] < index
                    assert value['thread_id'] == document['worker_joins'][0]['thread_id']
        assert set(starts) == set(ends) == set(WIDTHS) and len(received) == 1
        assert len(joins) == (1 if variant == 'optimized' else 0)
        for row in document['maps']:
            assert updates[row['width']] == row['actual_updates']
            assert peak[row['width']] == row['peak_storage_slots']
            assert starts[row['width']] < ends[row['width']] < received[0]
        assert [row['value'] for row in events if row['kind'] == 'joins'] == document['worker_joins']
    else:
        assert benchmark == 'pidigits'
        assert all(row['kind'] in {'pi', 'document'} for row in events)
        assert [row['value'] for row in events if row['kind'] == 'pi'] == document['extractions']
    return True
