"""Measure the retained-child traversal through the fixed deployment link."""

from collections import Counter
from importlib import import_module
import os
from pathlib import Path
from random import Random
from statistics import median
import sys

from build import BINARY, OUT as BUILD
from stage import BASE, HERE, INCUMBENT, PLAN, ROADMAP, read, sha, write

OUT = HERE / 'matched'
CONTROL = 'mech-control'
CANDIDATE = 'mech-inline-children'
PEERS = ('roc', 'libdeflate')
ACTORS = (CONTROL, CANDIDATE, *PEERS)
BOUNDARY = 'source_through_link'


def compare_workers():
    sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
    link = import_module('recheck-link')
    template = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
    output = HERE / 'linked'
    locations = {CONTROL: template, CANDIDATE: BUILD / 'rebuilt/native'}
    bindings = {}
    for name, artifact in locations.items():
        linked = output / 'worker-checks' / name
        binding = link.link_worker(template, artifact, linked)
        assert binding['worker_sha256'] == sha(template / 'mech-worker')
        link.check_worker(linked, f'inline-syntax-children-{name}')
        result = read(linked / 'qualification.json')
        assert result['complete_smokes'] == 162
        bindings[name] = binding
    assert link.check_contracts(output, tuple(locations)) == 0, (output / 'verify-linked.log').read_text()
    report = read(output / 'linked-proofs/result.json')
    assert report['matches_success_observations'] == 162 and len(report['checked_failure_cases']) == 6
    assert set(report['peak_stack_bytes'].values()) == {5280}
    return bindings, report


def intervals(values):
    random = Random(36730713)
    size = len(values)
    trials = sorted(median(values[random.randrange(size)] for _ in values) for _ in range(8000))
    return {'median': median(values), 'lower_90': trials[len(trials)//20],
            'upper_90': trials[len(trials)*19//20]}


def main():
    build = read(BUILD / 'result.json')
    plan = read(PLAN)
    assert build['comparison_plan_sha256'] == sha(PLAN)
    assert build['compiler_sha256'] == sha(BINARY) and build['base'] == BASE
    fixed = plan['settings']
    control = fixed[INCUMBENT]
    assert control['artifact_sha256'] == build['artifacts']
    candidate = control | {'compile': [str(BINARY), *control['compile'][1:]],
                           'compiler_sha256': build['compiler_sha256']}
    actors = {CONTROL: control, CANDIDATE: candidate, **{peer: fixed[peer] for peer in PEERS}}
    for actor in actors.values():
        assert sha(actor['compile'][0]) == actor['compiler_sha256']
        for path, expected in actor['host_inputs_sha256'].items():
            assert sha(path) == expected, path
    binder, checked = compare_workers()
    driver = PLAN.with_name('timing-driver.py')
    assert sha(driver) == plan['timer_sha256']
    text = driver.read_text()
    scope = {'__file__': str(driver), '__name__': 'owned_child_traversal_timing'}
    exec(compile(text, str(driver), 'exec'), scope)
    compile_and_link = scope['compile_and_link']
    # In this balanced cycle each compiler occupies each position once and
    # every directed adjacent pair is present once.
    cycle = [[ACTORS[(i + shift) % len(ACTORS)] for i in (0, 1, 3, 2)]
             for shift in range(len(ACTORS))]
    schedule = cycle + list(reversed(cycle)) + cycle
    assert set(Counter(edge for order in schedule for edge in zip(order, order[1:])).values()) == {3}
    OUT.mkdir()
    (OUT / 'timing-driver.py').write_text(text)
    write(OUT / 'plan.json', {'base': BASE, 'compiler_receipt_sha256': sha(BUILD / 'result.json'),
          'clock': 'fresh-process source through final link with prebuilt deployment support',
          'driver_sha256': sha(OUT / 'timing-driver.py'), 'inputs': actors, 'schedule': schedule,
          'linked_worker_sha256': binder[CANDIDATE]['worker_sha256'], 'load_before': os.getloadavg()})

    def sample(role, name):
        actor = actors[role]
        return compile_and_link(role, OUT / name / role,
                                actor['compile'], actor['link'], actor['reference_object'],
                                actor['flags'], actor['artifact_sha256'])

    warmup = {role: sample(role, 'warmup') for role in ACTORS}
    before = warmup[CONTROL]['stripped_executable']
    assert before == warmup[CANDIDATE]['stripped_executable'] and before['bytes'] == 292568
    runs = []
    loads = []
    for index, order in enumerate(schedule, 1):
        result = {role: sample(role, f'order-{index}') for role in order}
        assert all(result[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in ACTORS)
        runs.append(result)
        loads.append(os.getloadavg())
    medians = {role: {stage: median(row[role]['seconds'][stage] for row in runs)
                      for stage in warmup[role]['seconds']} for role in ACTORS}
    phases = {role: [{item['phase']: item['seconds'] for item in row[role]['compiler_phases']} for row in runs]
              for role in (CONTROL, CANDIDATE)}
    wall = [(row[CONTROL]['seconds'][BOUNDARY] - row[CANDIDATE]['seconds'][BOUNDARY]) * 1000 for row in runs]
    cpu = [(sum(row[CONTROL]['phase_cpu_seconds']['source'].values())
            - sum(row[CANDIDATE]['phase_cpu_seconds']['source'].values())) * 1000 for row in runs]
    stage_savings = {
        phase: intervals([(control[phase] - candidate[phase]) * 1000
                          for control, candidate in zip(phases[CONTROL], phases[CANDIDATE], strict=True)])
        for phase in phases[CONTROL][0]
    }
    report = {'plan_sha256': sha(OUT / 'plan.json'), 'warmup': warmup, 'observations': runs,
             'loads_after_blocks': loads, 'median_seconds': medians,
             'compiler_phases_seconds': {role: {phase: median(row[phase] for row in cases)
                                               for phase in cases[0]} for role, cases in phases.items()},
             'paired_ratios': {role: median(row[CANDIDATE]['seconds'][BOUNDARY] / row[role]['seconds'][BOUNDARY]
                                            for row in runs) for role in (CONTROL, *PEERS)},
             'paired_source_through_link_ms': intervals(wall), 'paired_source_cpu_ms': intervals(cpu),
             'paired_phase_savings_ms': stage_savings, 'linked_bindings': binder, 'linked_checks': checked}
    write(OUT / 'comparison.json', report)
    print({key: report[key] for key in ('paired_source_through_link_ms', 'paired_source_cpu_ms', 'paired_phase_savings_ms')},
          flush=True)


if __name__ == '__main__':
    main()
