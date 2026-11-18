"""Price compiler catalog overlap with an on/off comparison in one executable."""

from collections import Counter
from importlib import import_module
import os
from pathlib import Path
import runpy
from statistics import median
import sys

from measure import BOUNDARY, intervals
from stage import BASE, HERE, INCUMBENT, PLAN, ROADMAP, read, sha, write

parameters = runpy.run_path(str(HERE / 'catalog-build.py'), run_name='catalog_overlap_build')
CATALOG, MODE = (parameters[key] for key in ('OUT', 'MODE'))
OUT = CATALOG / 'matched'
CONTROL = 'mech-incumbent'
SERIAL = 'mech-serial-catalog'
PARALLEL = 'mech-overlapped-catalog'
MECH = (CONTROL, SERIAL, PARALLEL)
PEERS = ('roc', 'libdeflate')
ROLES = (*MECH, *PEERS)


def main():
    build = read(CATALOG / 'result.json')
    binary = CATALOG / 'compile_profile'
    assert build['base'] == BASE and build['compiler_sha256'] == sha(binary)
    reference = read(PLAN)['settings']
    control = reference[INCUMBENT]
    assert control['artifact_sha256'] == build['artifacts']
    serial = control | {'compile': [str(binary), *control['compile'][1:]], 'compiler_sha256': sha(binary)}
    parallel = serial | {'flags': serial['flags'] | {MODE: '1'}}
    actors = {CONTROL: control, SERIAL: serial, PARALLEL: parallel,
              **{peer: reference[peer] for peer in PEERS}}
    for value in actors.values():
        assert sha(value['compile'][0]) == value['compiler_sha256']
        for path, expected in value['host_inputs_sha256'].items():
            assert sha(path) == expected
    driver = PLAN.with_name('timing-driver.py')
    assert sha(driver) == read(PLAN)['timer_sha256']
    scope = {'__file__': str(driver), '__name__': 'catalog_deployment_timing'}
    text = driver.read_text()
    exec(compile(text, str(driver), 'exec'), scope)
    emit = scope['compile_and_link']
    prior = read(HERE / 'linked/linked-proofs/result.json')
    assert prior['matches_success_observations'] == 162 and len(prior['checked_failure_cases']) == 6
    sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
    link = import_module('recheck-link')
    template = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
    checked = CATALOG / 'worker'
    bound = link.link_worker(template, CATALOG / 'overlap/frontend', checked)
    assert bound['worker_sha256'] == sha(template / 'mech-worker')
    link.check_worker(checked, 'preloaded-catalog-verified')
    assert read(checked / 'qualification.json')['complete_smokes'] == 162

    cycle = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 1, 4, 2, 3)]
             for shift in range(len(ROLES))]
    schedule = cycle + [list(reversed(block)) for block in reversed(cycle)]
    assert set(Counter(pair for block in schedule for pair in zip(block, block[1:])).values()) == {2}
    OUT.mkdir()
    (OUT / 'timing-driver.py').write_text(text)
    write(OUT / 'plan.json', {'base': BASE, 'build_sha256': sha(CATALOG / 'result.json'),
        'driver_sha256': sha(OUT / 'timing-driver.py'), 'actors': actors, 'schedule': schedule,
        'phase_catalog_scope': 'serial catalog build or the join wait after the parser',
        'overlapped_work_field': 'catalog_work_seconds', 'load_before': os.getloadavg()})

    def run(role, label):
        actor = actors[role]
        output = OUT / label / role
        timing = emit(role, output, actor['compile'], actor['link'], actor['reference_object'],
                      actor['flags'], actor['artifact_sha256'])
        if role in (SERIAL, PARALLEL):
            receipt = read(output / 'frontend/compile.json')
            assert receipt['catalog_overlapped_parse'] == (role == PARALLEL)
            timing['catalog_work_seconds'] = receipt['catalog_work_seconds']
        return timing

    warmup = {role: run(role, 'warmup') for role in ROLES}
    reference_executable = warmup[CONTROL]['stripped_executable']
    assert reference_executable['bytes'] == 292568
    assert all(warmup[role]['stripped_executable'] == reference_executable for role in MECH)
    observed = []
    loads = []
    for index, order in enumerate(schedule, 1):
        row = {role: run(role, f'order-{index}') for role in order}
        assert all(row[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in ROLES)
        observed.append(row)
        loads.append(os.getloadavg())

    medians = {role: {phase: median(row[role]['seconds'][phase] for row in observed)
                for phase in warmup[role]['seconds']} for role in ROLES}
    phases = {role: [{item['phase']: item['seconds'] for item in row[role]['compiler_phases']} for row in observed]
              for role in MECH}
    comparison = {}
    for role in ROLES:
        if role == PARALLEL:
            continue
        elapsed = [(row[role]['seconds'][BOUNDARY] - row[PARALLEL]['seconds'][BOUNDARY]) * 1000
                    for row in observed]
        cpu = [(sum(row[role]['phase_cpu_seconds']['source'].values())
                - sum(row[PARALLEL]['phase_cpu_seconds']['source'].values())) * 1000 for row in observed]
        comparison[role] = {'through_link': intervals(elapsed), 'source_cpu': intervals(cpu),
            'ratio': median(row[PARALLEL]['seconds'][BOUNDARY] / row[role]['seconds'][BOUNDARY]
                            for row in observed)}
    data = {'plan_sha256': sha(OUT / 'plan.json'), 'median_seconds': medians,
            'phase_seconds': {role: {phase: median(row[phase] for row in cases)
                              for phase in cases[0]} for role, cases in phases.items()},
            'catalog_worker_seconds': {role: median(row[role]['catalog_work_seconds'] for row in observed)
                                       for role in (SERIAL, PARALLEL)},
            'comparisons': comparison, 'observations': observed, 'warmup': warmup,
            'checked_worker': bound, 'linked_failure_reference': prior, 'loads_after_blocks': loads}
    write(OUT / 'comparison.json', data)
    print({name: {'seconds': medians[name][BOUNDARY], 'comparison': comparison.get(name)}
           for name in ROLES}, flush=True)


if __name__ == '__main__':
    main()
