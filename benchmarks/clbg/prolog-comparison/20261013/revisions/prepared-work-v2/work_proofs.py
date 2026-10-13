"""Unexecuted independent reconciliation of pinned actual Prolog observer bytes."""
from collections import Counter
import json
from pathlib import Path

WIDTHS = (1,2,3,4,6,12,18)


def integer(value, minimum=0):
    assert type(value) is int and value >= minimum
    return value


def ordered_single_worker(trace):
    assert trace and len({value['worker'] for value in trace}) == 1
    assert type(trace[0]['worker']) is str and trace[0]['worker']
    for index,value in enumerate(trace,1):
        assert integer(value['ordinal'],1) == index


def events(path):
    result = []
    with Path(path).open() as stream:
        for line in stream:
            assert line.strip(), 'No omitted/blank native event records'
            value = json.loads(line)
            assert isinstance(value, dict)
            assert integer(value['ordinal'], 1) == len(result) + 1
            assert type(value['worker']) is str and value['worker']
            result.append(value)
    assert result and len({value['worker'] for value in result}) == 1
    return result


def graph(value, depth, check):
    integer(depth)
    assert set(value) == {'root','nodes','distinct_nodes'}
    root = integer(value['root'], 1)
    rows = value['nodes']
    assert isinstance(rows, list)
    assert integer(value['distinct_nodes'], 1) == len(rows)
    nodes = {}
    for row in rows:
        assert set(row) == {'id','left','right','leaf'}
        identifier = integer(row['id'], 1)
        assert identifier not in nodes
        integer(row['left']); integer(row['right'])
        assert type(row['leaf']) is bool
        nodes[identifier] = row
    visited = set()
    def visit(identifier, remaining):
        assert identifier in nodes and identifier not in visited, 'Shared or cyclic actual child'
        visited.add(identifier)
        row = nodes[identifier]
        if remaining == 0:
            assert row['leaf'] is True and row['left'] == row['right'] == 0
            return 1
        assert row['leaf'] is False and row['left'] > 0 and row['right'] > 0
        return 1 + visit(row['left'], remaining-1) + visit(row['right'], remaining-1)
    actual_count = visit(root, depth)
    assert visited == set(nodes) and integer(check, 1) == actual_count == len(rows)
    return actual_count


def trees(trace, n):
    ordered_single_worker(trace)
    maximum = max(6, integer(n))
    assert trace[0]['event'] == 'stretch_checked' and trace[0]['depth'] == maximum+1
    assert trace[0]['retained_across_callback'] is False
    graph(trace[0]['graph'], maximum+1, trace[0]['check'])
    assert trace[1]['event'] == 'retain' and trace[1]['depth'] == maximum
    retained = trace[1]['graph']
    graph(retained, maximum, len(retained['nodes']))
    position = 2
    intervals = []
    for depth in range(4, maximum+1, 2):
        iterations = 1 << (maximum-depth+4)
        checks = 0
        for remaining in range(iterations, 0, -1):
            begin, finished = trace[position:position+2]
            assert begin['event'] == 'temporary_begin' and finished['event'] == 'temporary_checked'
            assert integer(begin['remaining'],1) == integer(finished['remaining'],1) == remaining
            assert integer(begin['depth']) == integer(finished['depth']) == depth
            assert begin['worker'] == finished['worker']
            assert finished['concurrent_retained_disjoint'] is True
            checks += graph(finished['graph'], depth, finished['check'])
            intervals.append(dict(begin=begin['ordinal'], end=finished['ordinal'],
                                  remaining=remaining, depth=depth, worker=begin['worker']))
            position += 2
        assert checks > 0
    assert position == len(trace)-1
    final = trace[position]
    assert final['event'] == 'retained_checked' and final['depth'] == maximum
    assert final['actual_root_identity'] is final['all_actual_node_identities'] is True
    assert final['graph'] == retained
    graph(final['graph'], maximum, final['check'])
    return dict(status='passed', actual_stretch_retained_and_all_iteration_graphs=True,
        each_real_leaf_and_interior_observed=True, original_check_counts_reconciled=True,
        callback_same_term_retained_identities=True, actual_iteration_intervals=intervals,
        single_worker=True, native_allocator_review_required_separately=True, chart_admission=False)


def encoded(word):
    value = 0
    for base in word:
        value = (value << 2) + 'ACGT'.index(base)
    return value


def maps(trace, expected_sequence):
    ordered_single_worker(trace)
    assert type(expected_sequence) is str and set(expected_sequence.upper()) <= set('ACGT')
    sequence = expected_sequence.upper()
    assert len(trace) == 15
    outputs = []
    for index, width in enumerate(WIDTHS):
        begin, finished = trace[2*index:2*index+2]
        assert begin['event'] == 'map_begin' and finished['event'] == 'map_completed'
        assert integer(begin['map_id'],1) == integer(finished['map_id'],1) == index+1
        assert integer(begin['width'],1) == integer(finished['width'],1) == width
        assert begin['worker'] == finished['worker']
        assert integer(begin['size']) == integer(begin['capacity']) == 0
        assert integer(begin['bucket_arity'],1) == 1
        assert type(finished['sequence']) is str and finished['sequence'].upper() == sequence
        assert integer(finished['sequence_length']) == len(sequence)
        assert finished['actual_begin_root_identity'] is True
        assert finished['per_window_trace_recorded'] is False
        expected = Counter(encoded(sequence[start:start+width])
                           for start in range(max(0,len(sequence)-width+1)))
        actual = {}
        for entry in finished['entries']:
            assert set(entry) == {'key','value'}
            key, count = integer(entry['key']), integer(entry['value'],1)
            assert key not in actual and key < 1 << (2*width)
            actual[key] = count
        assert actual == expected, 'Actual complete map or overlapping/tail count differs'
        assert integer(finished['size']) == len(actual)
        capacity = integer(finished['capacity'])
        assert integer(finished['bucket_arity'],1) == 2*capacity+1
        if actual:
            assert capacity >= 4 and capacity >= 2*len(actual)
            assert capacity & (capacity-1) == 0
        else:
            assert capacity == 0
        windows = max(0,len(sequence)-width+1)
        assert sum(actual.values()) == windows
        outputs.append(dict(width=width, distinct_keys=len(actual), capacity=capacity,
                            complete_windows=windows, source_growth_observed=capacity > 0))
    final = trace[-1]
    assert final['event'] == 'maps_returned'
    assert final['map_ids'] == list(range(1,8)) and all(type(i) is int for i in final['map_ids'])
    assert final['actual_all_root_identities'] is True
    assert final['single_worker'] is True and final['joins_required'] is False
    return dict(status='passed', actual_seven_complete_maps=True, maps=outputs,
        complete_map_counts_and_window_totals=True, actual_begin_return_root_identities=True,
        per_window_trace_recorded=False, exact_original_source_loop_review_required=True, chart_admission=False)


def pi(trace, n, expected_digits):
    ordered_single_worker(trace)
    integer(n)
    assert trace[0]['event'] == 'pi_begin' and integer(trace[0]['requested']) == n
    assert trace[-1]['event'] == 'pi_finished' and integer(trace[-1]['requested']) == n
    q,r,t,k,count = 1,0,1,1,0
    digits = []
    for candidate in trace[1:-1]:
        assert candidate['event'] == 'pi_candidate'
        assert integer(candidate['requested']) == n and integer(candidate['count']) == count
        for field, value in (('q',q),('r',r),('t',t),('k',k)):
            assert type(candidate[field]) is int and candidate[field] == value
        n3, n4 = 3*q+r, 4*q+r
        for field,value in (('numerator3',n3),('numerator4',n4),('quotient3',n3//t),('quotient4',n4//t)):
            assert type(candidate[field]) is int and candidate[field] == value
        accepted = n3//t == n4//t
        assert candidate['accepted'] is accepted
        if accepted:
            digit = n3//t
            assert 0 <= digit <= 9 and count < n
            digits.append(str(digit))
            q,r = q*10,10*(r-digit*t)
            count += 1
        else:
            factor = 2*k+1
            q,r,t,k = q*k,(2*q+r)*factor,t*factor,k+1
    assert integer(trace[-1]['observed_accepted']) == count == n
    assert ''.join(digits) == expected_digits and len(expected_digits) == n
    return dict(status='passed', actual_both_division_operands_and_quotients=True,
        independent_unbounded_recurrence=True, actual_candidates=len(trace)-2,
        observed_digits=n, includes_accepted_and_rejected=(any(c.get('accepted') is False for c in trace)),
        chart_admission=False)
