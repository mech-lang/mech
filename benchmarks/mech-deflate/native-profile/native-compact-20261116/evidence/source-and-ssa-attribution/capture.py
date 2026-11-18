"""Measure source stages and SSA occupancy on the unchanged compressor."""

from collections import Counter, defaultdict
import itertools
import json
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter

from build import COMPILER, MATCHED, OUTPUT, REFERENCE, read, save, sha
from stage import HEAD, HERE

OUT = HERE / 'observed'
METRICS = ('empty', 'one', 'two_to_four', 'five_to_fifteen', 'sixteen_to_sixty_three', 'sixty_four_or_more',
           'entries', 'dense_maps', 'sparse_growths', 'dense_growths', 'definitions', 'use_requests',
           'local_hits', 'nonlocal_requests', 'definition_samples', 'definition_ns', 'use_samples', 'use_ns')
PROBES = {'plain': {}, 'source': {'MECH_PROFILE_SOURCE_SEMANTICS': '1'},
          'ssa': {'MECH_PROFILE_SSA_CONSTRUCTION': '1'}}
ORDERS = tuple(itertools.permutations(PROBES))


def acquire(plan, label, flags, binary):
    location = OUT / label
    location.mkdir(parents=True)
    command = [binary, plan['source'], location / 'emitted', 'compact']
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    start = perf_counter()
    with (location / 'phases.log').open('x') as log:
        subprocess.run(command, env=env | plan['native_flags'] | flags, check=True, stdout=log, stderr=subprocess.STDOUT)
    elapsed = perf_counter() - start
    paths = {'bytecode': 'program.mecb', 'encoder': 'program.o', 'metadata': 'metadata.json'}
    artifacts = {name: sha(location / 'emitted' / filename) for name, filename in paths.items()}
    assert artifacts == plan['artifacts']
    phases = stages(location / 'phases.log')
    result = {'command': list(map(str, command)), 'elapsed': elapsed, 'artifacts': artifacts,
              'log_sha256': sha(location / 'phases.log'), 'measurements': phases}
    save(location / 'result.json', result)
    return result


def stages(log):
    outer = {}
    source = Counter()
    catalog = Counter()
    semantic_lookup = []
    ssa = []
    words = []
    bank = None
    for line in log.read_text().splitlines():
        key, _, rest = line.partition(' ')
        if line.startswith('{"phase":'):
            phase = json.loads(line)
            outer[phase['phase']] = phase['seconds']
        elif key == 'profile-source-semantic':
            name, ns = rest.split()
            source[name] += int(ns)
        elif key == 'profile-source-requirement-lookups':
            lookups, comparisons, ns = map(int, rest.split())
            semantic_lookup.append({'lookups': lookups, 'comparisons': comparisons, 'ns': ns})
        elif key == 'profile-source-catalog':
            name, ns = rest.split()
            catalog[name] += int(ns)
        elif key == 'profile-ssa':
            ssa.append(dict(zip(METRICS, map(int, rest.split()), strict=True)))
        elif key == 'profile-word-bank':
            entry, count = map(int, rest.split())
            bank = {'entry': entry, 'count': count, 'data': []}
            words.append(bank)
        elif key == 'profile-word':
            index, origin, reads, writes, params = rest.split()
            assert bank is not None
            bank['data'].append({'slot': int(index), 'origin': origin, 'reads': int(reads),
                                 'writes': int(writes), 'parameter_reads': int(params)})
    assert {'catalog','frontend','parse','native'} <= set(outer)
    if words:
        assert all(row['count'] == len(row['data']) for row in words)
    return {'outer_seconds': outer, 'source_ns': dict(source), 'catalog_ns': dict(catalog),
            'requirement_lookups': semantic_lookup, 'ssa_builders': ssa, 'word_banks': words}


def conclude(trials):
    source = [order['source']['measurements'] for order in trials]
    native = [order['ssa']['measurements'] for order in trials]
    plain = [order['plain']['measurements'] for order in trials]
    assert all(len(m['requirement_lookups']) == 1 and m['source_ns'] and m['catalog_ns'] for m in source)
    assert all(m['ssa_builders'] and m['word_banks'] for m in native)
    def medians(rows):
        keys = rows[0].keys()
        assert all(row.keys() == keys for row in rows)
        return {name: median(row[name] for row in rows) for name in keys}
    ssa = [{name: sum(row[name] for row in run['ssa_builders']) for name in METRICS} for run in native]
    roots = [max(run['ssa_builders'], key=lambda row: row['use_requests']) for run in native]
    lane_groups = []
    for run in native:
        root = max(run['word_banks'], key=lambda row: row['count'])
        grouped = defaultdict(Counter)
        for lane in root['data']:
            key = lane['origin'] + ('_never_written' if lane['writes'] == 0 else '_written')
            grouped[key].update({'lanes': 1, 'reads': lane['reads'], 'writes': lane['writes'],
                                 'parameter_reads': lane['parameter_reads']})
        lane_groups.append({key: dict(value) for key, value in grouped.items()})
    assert all(x == lane_groups[0] for x in lane_groups)
    stages_sec = {kind: medians([row['outer_seconds'] for row in runs])
                  for kind, runs in [('disabled', plain), ('source', source), ('ssa', native)]}
    return {'source_ns': medians([run['source_ns'] for run in source]),
            'catalog_ns': medians([run['catalog_ns'] for run in source]),
            'requirement_lookup': medians([run['requirement_lookups'][0] for run in source]),
            'ssa_aggregate': medians(ssa), 'ssa_largest_builder': medians(roots),
            'largest_root_word_lanes': lane_groups[0], 'outer_stage_medians_seconds': stages_sec}


def main():
    OUT.mkdir(exist_ok=False)
    source = read(MATCHED)['actors'][REFERENCE]
    assert sha(Path(source['compile'][0])) == source['compiler_sha256']
    build = read(OUTPUT / 'build.json')
    assert build['base'] == HEAD and build['compiler_sha256'] == sha(COMPILER)
    plan = {'source_pin': HEAD, 'source': source['compile'][1], 'source_sha256': sha(Path(source['compile'][1])),
            'artifacts': source['artifact_sha256'], 'native_flags': source['flags'],
            'repeated_modes': PROBES, 'orders': ORDERS, 'known_release': source['compile'][0],
            'known_release_sha256': source['compiler_sha256'], 'probe_compiler_sha256': sha(COMPILER)}
    save(OUT / 'plan.json', plan)
    preflight = acquire(plan, 'maintained-reference', {}, plan['known_release'])
    trials = []
    for index, permutation in enumerate(ORDERS, 1):
        row = {role: acquire(plan, f'round-{index}/{role}', PROBES[role], COMPILER) for role in permutation}
        trials.append(row)
        print('Completed attribution block', index, 'of', len(ORDERS), flush=True)
    result = conclude(trials)
    result.update(plan_sha256=sha(OUT / 'plan.json'), reference=preflight, trials=trials)
    save(OUT / 'summary.json', result)
    print(json.dumps({key: value for key, value in result.items() if key not in ('reference', 'trials')}, indent=2), flush=True)


if __name__ == '__main__':
    main()
