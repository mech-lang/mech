"""Emit and screen scorer variants with the existing frozen workers and case inventory."""

from collections import defaultdict
from importlib import import_module
import sys

from prepare import BASE, HERE
from build import COMPILER, OUT

PREVIOUS = HERE.parent / 'scorer-runtime'
sys.path.insert(0, str(PREVIOUS))
driver = import_module('emit')
driver.HERE = HERE
driver.PIN = BASE
rebase = import_module('rebase-layout')
scheduler = import_module('schedule')
count = import_module('count')
fit = import_module('fresh-profile')
read, save, sha = driver.read, driver.save, driver.sha

ABI = {'MECH_COMPOSE_ADDRESS_ARGUMENTS': '1'}
FIRST = {'MECH_ORDERED_BRANCH_WINNER': 'first'}
ALL = {'MECH_ORDERED_BRANCH_WINNER': 'all'}
ARMS = {
    'ord-control': {},
    'ord-abi': ABI,
    'ord-first': FIRST,
    'ord-all': ALL,
    'ord-abi-first': ABI | FIRST,
    'ord-abi-all': ABI | ALL,
}


def emit(role, knobs, profile=None):
    receipt = read(OUT / 'result.json')
    assert receipt['base'] == BASE and sha(COMPILER) == receipt['compiler_sha256']
    driver.emit(role, driver.HORIZONTAL | driver.REVERSE | knobs, COMPILER,
                profile_path=profile, drop_flags=('MECH_COMPACT_ROOT_TAIL_PROFILE',))
    metadata = read(HERE / role / 'bound/metadata.json')
    by_name = [r['composed_variant'] for r in metadata['numeric_phases']
               if r.get('composed_variant', {}).get('symbol') == driver.SCORER[1:]]
    phase, = by_name
    addresses = phase['admission'].get('address_arguments')
    assert bool(addresses) == ('MECH_COMPOSE_ADDRESS_ARGUMENTS' in knobs)
    print(role, 'root/scorer stack bytes', metadata['stack_bytes'], phase['stack_bytes'], flush=True)


def measure_counts(role):
    sys.argv = [sys.argv[0], role]
    count.main()


def profiles(role):
    shape = rebase.parse(role)
    sample = HERE / role / 'instruction-counts/counts.json'
    training = read(sample)
    assert training['object_sha256'] == shape['object_sha256']
    assert training['symbol'] == driver.SCORER and training['literal_offset'] == shape['pool']
    heads = shape['heads']
    index = {head: i for i, head in enumerate(heads)}
    graph = [tuple(dict.fromkeys(dest for dest in row[1:] if dest is not None)) for row in shape['blocks']]
    weights = defaultdict(float)
    hotness = [0.] * len(graph)
    traces = {}
    for case, row in training['cases'].items():
        counts = {index[int(block['address'], 16) - shape['atom']['start']]: block['visits']
                  for block in row['blocks']}
        peak = max(counts.values())
        trace = scheduler.edge_counts(graph, counts)
        for at, n in counts.items():
            hotness[at] += n / peak
        for edge, n in trace.items():
            weights[edge] += n / peak
        traces[case] = trace
    ranked = {}
    for src, destinations in enumerate(graph):
        for dst in destinations:
            fallthrough = shape['blocks'][src][2]
            potential = weights[src, fallthrough] if len(destinations) == 2 else weights[src, dst]
            ranked[src, dst] = potential + .35 * weights[src, dst] + .001
    order = fit.refine(scheduler.order_by_weight(ranked, hotness), ranked)
    tuned = HERE / f'{role}-scorer-profile.json'
    save(tuned, {'order': [heads[block] for block in order], 'literal_offset': shape['pool']})
    prediction = {
        case: {'before': scheduler.overhead(shape['blocks'], list(range(len(graph))), trace),
               'after': scheduler.overhead(shape['blocks'], order, trace)}
        for case, trace in traces.items()
    }
    save(HERE / role / 'layout-proof.json',
         {'profile_sha256': sha(tuned), 'training_sha256': sha(sample),
          'shape': shape, 'expected_unconditional_branches': prediction})
    print(role, prediction, flush=True)
    return tuned


def main():
    action, *roles = sys.argv[1:]
    if action in ('emit', 'layout'):
        for role in roles:
            if action == 'layout':
                measure_counts(role)
                tuned = profiles(role)
                emit(role + '-layout', ARMS[role], tuned)
            else:
                emit(role, ARMS[role])
    elif action == 'count':
        for role in roles:
            measure_counts(role)
    else:
        modules = {'focus': 'timing', 'full': 'timing', 'check': 'qualify', 'matched': 'matched'}
        role, = roles
        sys.argv = [sys.argv[0], role, action] if action in ('full', 'focus') else [sys.argv[0], role]
        import_module(modules[action]).main()


if __name__ == '__main__':
    main()
