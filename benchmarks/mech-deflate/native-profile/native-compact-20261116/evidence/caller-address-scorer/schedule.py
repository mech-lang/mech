"""Schedule scorer blocks using the exact training edge counts."""
from collections import defaultdict
from importlib import import_module
import sys

from emit import HERE, HORIZONTAL, REVERSE, HISTORY, SCORER, Q, emit, read, save, sha, symbols

rebase = import_module('rebase-layout')
COUNTS = Q / 'matcher-dynamic-blocks/cranelift-horizontal-minimum-3009-no-growth-composed/counts.json'


def edge_counts(graph, entries):
    counts = defaultdict(int, entries)
    preds = defaultdict(list)
    for origin, destinations in enumerate(graph):
        for dest in destinations:
            preds[dest].append(origin)
    rows = [
        (counts[origin], [(origin, dest) for dest in destinations])
        for origin, destinations in enumerate(graph) if destinations
    ] + [
        (counts[dest], [(origin, dest) for origin in sources])
        for dest, sources in preds.items()
    ]
    resolved = {}
    while True:
        before = len(resolved)
        for count, edges in rows:
            remaining = count - sum(resolved.get(edge, 0) for edge in edges)
            assert remaining >= 0
            unknown = [edge for edge in edges if edge not in resolved]
            if not unknown:
                assert remaining == 0
            elif not remaining:
                resolved.update(dict.fromkeys(unknown, 0))
            elif len(unknown) == 1:
                resolved[unknown[0]] = remaining
        if len(resolved) == before:
            break
    assert len(resolved) == sum(map(len, graph))
    return resolved


def order_by_weight(weights, hotness):
    chains = {at: [at] for at in range(len(hotness))}
    origins = list(chains)
    for (src, dest), weight in sorted(weights.items(), key=lambda e: (-e[1], *e[0])):
        if not weight or dest == 0:
            continue
        left, right = origins[src], origins[dest]
        if left == right or chains[left][-1] != src or right != dest:
            continue
        group = chains.pop(right)
        chains[left].extend(group)
        for block in group:
            origins[block] = left
    tails = sorted((head for head in chains if head != 0),
                   key=lambda head: (-max(hotness[node] for node in chains[head]), head))
    return [node for head in [0, *tails] for node in chains[head]]


def overhead(blocks, order, flows):
    total = 0
    for at, following in zip(order, [*order[1:], None], strict=True):
        kind, target, next_block = blocks[at]
        if next_block is None:
            if target is not None and target != following:
                total += flows[at, target]
        elif target is None:
            if next_block != following:
                total += flows[at, next_block]
        elif target != following and next_block != following:
            total += flows[at, next_block]
    return total


def main():
    before, after = rebase.parse('horizontal'), rebase.parse('horizontal-reverse')
    assert before['blocks'] == after['blocks']
    training = read(COUNTS)
    assert training['object_sha256'] == sha(HISTORY / 'program.o') and training['symbol'] == SCORER
    archived = symbols(HISTORY / 'program.o')['symbols'][SCORER]
    assert archived['sha256'] == before['atom']['sha256']
    assert archived['relocations'] == before['atom']['relocations']
    assert training['literal_offset'] == before['pool']
    index = {head: i for i, head in enumerate(before['heads'])}
    graph = [tuple(dict.fromkeys(n for n in row[1:] if n is not None)) for row in before['blocks']]
    assert all(len(row) <= 2 for row in graph)
    totals = defaultdict(float)
    hotness = [0.] * len(graph)
    cases = {}
    for case, record in training['cases'].items():
        if not record['calls']:
            continue
        counts = {index[int(row['address'], 16) - archived['start']]: row['visits']
                  for row in record['blocks']}
        peak = max(counts.values())
        edges = edge_counts(graph, counts)
        for block, count in counts.items():
            hotness[block] += count / peak
        for label, flow in edges.items():
            totals[label] += flow / peak
        cases[case] = edges
    weights = {}
    for at, destinations in enumerate(graph):
        for dest in destinations:
            other = before['blocks'][at][2]
            # Adjacent successors avoid the emitted fallback jump. Break ties
            # in favor of its most commonly executed direction.
            penalty = totals[at, other] if len(destinations) == 2 else totals[at, dest]
            weights[at, dest] = penalty + .35 * totals[at, dest]
    new_order = order_by_weight(weights, hotness)
    previous = read(HERE / 'scorer-block-profile.json')
    old_order = [index[head] for head in previous['order']]
    predicted = {case: dict(balanced=overhead(before['blocks'], old_order, flow),
                            scheduled=overhead(before['blocks'], new_order, flow))
                 for case, flow in cases.items()}
    assert sorted(new_order) == list(range(len(graph))) and new_order[0] == 0
    path = HERE / 'scheduled-block-profile.json'
    save(path, {'order': [after['heads'][id] for id in new_order], 'literal_offset': after['pool']})
    save(HERE / 'schedule-proof.json', {'training_sha256': sha(COUNTS), 'profile_sha256': sha(path),
         'before': before, 'after': after, 'predicted_dynamic_unconditional_branches': predicted,
         'known_edges_per_case': {case: len(flow) for case, flow in cases.items()}})
    print(predicted, flush=True)
    if sys.argv[1:] == ['emit'] and new_order != old_order:
        emit('horizontal-reverse-scheduled', HORIZONTAL | REVERSE, profile_path=path)


if __name__ == '__main__':
    main()
