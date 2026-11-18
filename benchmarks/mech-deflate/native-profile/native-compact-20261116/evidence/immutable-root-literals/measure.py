"""Compare the literal-lowering probe with two controls and the fixed peers."""

from importlib import import_module
import os
from random import Random
from pathlib import Path
from statistics import median
import sys

from build import BASE, BINARY, HERE, MATCHED, MODE, OUTPUT, REFERENCE, ROADMAP, read, save, sha

OUT = HERE / 'matched'
INCUMBENT = 'mech-incumbent'
DISABLED = 'mech-literals-disabled'
ENABLED = 'mech-literals-enabled'
PEERS = ('roc', 'libdeflate')
MECH = (INCUMBENT, DISABLED, ENABLED)
ROLES = (*MECH, *PEERS)
BOUNDARY = 'source_through_link'


def binding():
    template = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
    sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
    support = import_module('recheck-link')
    target = HERE / 'linked'
    choices = {INCUMBENT: template, ENABLED: OUTPUT / 'enabled/frontend'}
    verified = {}
    for role, path in choices.items():
        folder = target / 'worker-checks' / role
        linked = support.link_worker(template, path, folder)
        support.check_worker(folder, f'entry-literal-{role}')
        report = read(folder / 'qualification.json')
        assert report['complete_smokes'] == 162
        verified[role] = {'link': linked, 'smoke': report}
    assert verified[INCUMBENT]['link']['worker_sha256'] == sha(template / 'mech-worker')
    assert support.check_contracts(target, tuple(choices)) == 0, (target / 'verify-linked.log').read_text()
    report = read(target / 'linked-proofs/result.json')
    assert report['matches_success_observations'] == 162 and len(report['checked_failure_cases']) == 6
    return verified, report


def timing_function():
    path = MATCHED.with_name('timing-driver.py')
    source = path.read_text()
    assert sha(path) == read(MATCHED)['timer_sha256']
    scope = {'__file__': str(path), '__name__': 'entry_literal_deployment_timing'}
    exec(compile(source, str(path), 'exec'), scope)
    return scope['compile_and_link'], source


def interval(samples):
    random = Random(1916314301)
    count = len(samples)
    draws = sorted(median(samples[random.randrange(count)] for _ in samples) for _ in range(8000))
    return {'median': median(samples), 'lower_90': draws[len(draws)//20], 'upper_90': draws[len(draws)*19//20]}


def main():
    build = read(OUTPUT / 'result.json')
    assert build['compiler_sha256'] == sha(BINARY)
    assert (build['base'], build['tests']['passed']) == (BASE, 2)
    direct_build = build['enabled']
    fixed = read(MATCHED)['actors']
    current = fixed[REFERENCE]
    assert build['disabled'] == current['artifact_sha256']
    assert all(direct_build['flags'].get(key) == value for key, value in current['flags'].items())
    commands = [str(BINARY), *current['compile'][1:]]
    actors = {INCUMBENT: current, DISABLED: current | {'compile': commands, 'compiler_sha256': sha(BINARY)},
              ENABLED: current | {'compile': commands, 'compiler_sha256': sha(BINARY),
                                   'flags': direct_build['flags'], 'artifact_sha256': direct_build['artifacts']}}
    actors.update({peer: fixed[peer] for peer in PEERS})
    for actor in actors.values():
        assert sha(actor['compile'][0]) == actor['compiler_sha256']
        for local, expected in actor['host_inputs_sha256'].items():
            assert sha(local) == expected
    # Build/tool setup and the checked calls are outside the source-to-link clock.
    links, checked = binding()
    same_object = build['disabled']['encoder'] == direct_build['artifacts']['encoder']
    assert (links[INCUMBENT]['link']['worker_sha256'] == links[ENABLED]['link']['worker_sha256']) == same_object
    emit, source = timing_function()
    cycle = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 2, 4, 1, 3)]
             for shift in range(len(ROLES))]
    order = cycle + [list(reversed(row)) for row in cycle]
    OUT.mkdir()
    (OUT / 'timing-driver.py').write_text(source)
    plan = {'source': BASE, 'probe_build_sha256': sha(OUTPUT / 'result.json'),
            'clock': 'fresh source through final link with prebuilt deployment support',
            'settings': actors, 'schedule': order, 'load_before': os.getloadavg(),
            'timer_sha256': sha(OUT / 'timing-driver.py'), 'exact_artifacts': build['exact']}
    save(OUT / 'plan.json', plan)

    def trial(role, label):
        actor = actors[role]
        output = OUT / label / role
        result = emit(role, output, actor['compile'], actor['link'], actor['reference_object'],
                      actor['flags'], actor['artifact_sha256'])
        if role == ENABLED:
            log = (output / 'source.log').read_text()
            assert 'direct scalar literals=' in log and 'direct literal attempt withdrew' not in log
        return result

    warmup = {role: trial(role, 'warmup') for role in ROLES}
    observations = []
    loads = []
    for i, row in enumerate(order, 1):
        observations.append({role: trial(role, f'order-{i}') for role in row})
        loads.append(os.getloadavg())
    for row in [warmup, *observations]:
        assert row[INCUMBENT]['stripped_executable'] == row[DISABLED]['stripped_executable']
        if build['exact']:
            assert row[INCUMBENT]['stripped_executable'] == row[ENABLED]['stripped_executable']
        assert all(row[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in ROLES)

    def phases(role):
        rows = [{p['phase']: p['seconds'] for p in row[role]['compiler_phases']} for row in observations]
        return {name: median(row[name] for row in rows) for name in rows[0]}

    def rates(candidate, sibling):
        elapsed = [(row[sibling]['seconds'][BOUNDARY] - row[candidate]['seconds'][BOUNDARY]) * 1000
                   for row in observations]
        cpu = [(sum(row[sibling]['phase_cpu_seconds']['source'].values())
                - sum(row[candidate]['phase_cpu_seconds']['source'].values())) * 1000 for row in observations]
        ratios = [row[candidate]['seconds'][BOUNDARY] / row[sibling]['seconds'][BOUNDARY] for row in observations]
        native = []
        if candidate in MECH and sibling in MECH:
            for row in observations:
                def compiled_native(actor):
                    return next(entry['seconds'] for entry in row[actor]['compiler_phases'] if entry['phase'] == 'native')
                native.append((compiled_native(sibling) - compiled_native(candidate)) * 1000)
        return {'elapsed_x': median(ratios), 'paired_elapsed_ms': elapsed, 'elapsed_interval_ms': interval(elapsed),
                'paired_cpu_ms': cpu, 'cpu_interval_ms': interval(cpu),
                'native_interval_ms': interval(native) if native else None}

    data = {'plan_sha256': sha(OUT / 'plan.json'), 'warmup': warmup,
            'observations': observations, 'load_after_blocks': loads, 'links': links, 'checked': checked,
            'median_seconds': {role: {phase:median(row[role]['seconds'][phase] for row in observations)
                                      for phase in warmup[role]['seconds']} for role in ROLES},
            'compiler_phases_seconds': {role: phases(role) for role in MECH},
            'enabled_comparisons': {role: rates(ENABLED, role) for role in ROLES if role != ENABLED},
            'disabled_vs_incumbent': rates(DISABLED, INCUMBENT)}
    save(OUT / 'comparison.json', data)
    print({role: result['elapsed_interval_ms'] for role, result in data['enabled_comparisons'].items()}, flush=True)


if __name__ == '__main__':
    main()
