"""Build a matched layout profile from the new caller-address scorer."""
from collections import defaultdict
from importlib import import_module
import sys

from addresses import FLAG, NO_STALE_PROFILE, OUT
from emit import HERE, HORIZONTAL, REVERSE, SCORER, emit, read, save, sha
from schedule import edge_counts, overhead, order_by_weight

rebase = import_module('rebase-layout')


def refine(order, weights):
    def score(items):
        return sum(weights.get(pair, 0) for pair in zip(items, items[1:]))

    current = order
    while True:
        value = score(current)
        best = current
        for begin in range(1, len(current)):
            for size in range(1, min(4, len(current) - begin) + 1):
                segment = current[begin:begin + size]
                remainder = current[:begin] + current[begin + size:]
                for dest in range(1, len(remainder) + 1):
                    if dest == begin:
                        continue
                    trial = remainder[:dest] + segment + remainder[dest:]
                    offered = score(trial)
                    if offered > value + 1e-9:
                        best, value = trial, offered
        if best == current:
            return current
        current = best


def main():
    role = 'caller-addresses'
    shape = rebase.parse(role)
    sample = HERE / role / 'instruction-counts/counts.json'
    observed = read(sample)
    assert observed['object_sha256'] == shape['object_sha256']
    assert observed['symbol'] == SCORER and observed['literal_offset'] == shape['pool']
    heads = shape['heads']
    numbers = {head: i for i, head in enumerate(heads)}
    graph = [tuple(dict.fromkeys(n for n in row[1:] if n is not None)) for row in shape['blocks']]
    weights = defaultdict(float)
    hotness = [0.] * len(graph)
    cases = {}
    for name, record in observed['cases'].items():
        assert record['calls'] > 0
        counts = {numbers[int(row['address'], 16) - shape['atom']['start']]: row['visits']
                  for row in record['blocks']}
        peak = max(counts.values())
        edges = edge_counts(graph, counts)
        for block, count in counts.items():
            hotness[block] += count / peak
        for pair, count in edges.items():
            weights[pair] += count / peak
        cases[name] = edges
    for at, destinations in enumerate(graph):
        after = shape['blocks'][at][2]
        if after in destinations:
            weights[at, after] += .001
    ordered = order_by_weight(weights, hotness)
    refined = 'refine' in sys.argv[1:]
    if refined:
        demand = {}
        for at, destinations in enumerate(graph):
            for dest in destinations:
                fallthrough = shape['blocks'][at][2]
                saving = weights[at, fallthrough] if len(destinations) == 2 else weights[at, dest]
                demand[at, dest] = saving + .35 * weights[at, dest] + .001
        ordered = refine(order_by_weight(demand, hotness), demand)
    label = 'caller-addresses-refined' if refined else 'caller-addresses-trained'
    prefix = 'caller-addresses-refined' if refined else 'caller-addresses'
    path = HERE / f'{prefix}-block-profile.json'
    save(path, {'order': [heads[i] for i in ordered], 'literal_offset': shape['pool']})
    expected = {name: {'unprofiled': overhead(shape['blocks'], list(range(len(graph))), trace),
                       'profiled': overhead(shape['blocks'], ordered, trace)}
                for name, trace in cases.items()}
    save(HERE / f'{prefix}-layout-proof.json', {'profile_sha256': sha(path),
         'training_sha256': sha(sample), 'shape': shape, 'expected_unconditional_branches': expected})
    print(expected, flush=True)
    if 'emit' in sys.argv[1:]:
        compiler = OUT / 'compile_profile'
        assert sha(compiler) == read(OUT / 'build.json')['compiler_sha256']
        emit(label, HORIZONTAL | REVERSE | FLAG, compiler,
             profile_path=path, drop_flags=NO_STALE_PROFILE)


if __name__ == '__main__':
    main()
