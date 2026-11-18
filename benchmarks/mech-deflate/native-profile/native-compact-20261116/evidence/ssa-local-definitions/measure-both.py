"""Compare both SSA changes with fresh linked builds and child CPU accounting."""

from importlib import import_module
import os
from pathlib import Path
from statistics import median
import sys

previous = import_module('measure-local-hit')
BASE, HERE, Q, PLAN = previous.BASE, previous.HERE, previous.Q, previous.PLAN
read, save, sha = previous.read, previous.save, previous.sha
CONTROL = 'mech-release'
CHANGES = {'mech-ssa-local': 'local-hit', 'mech-ssa-first-definition': 'first-definition'}
PEERS = ('roc', 'libdeflate')
ROLES = (CONTROL, *CHANGES, *PEERS)
MATCHED = HERE / 'matched-both'
DRIVER = HERE.parent / 'measure-deployment-builds.py'


def measure_function():
    source = DRIVER.read_text()
    updates = (
        ("    rss = {}\n", "    rss = {}\n    cpu_seconds = {}\n"),
        ("            rss[phase] = usage.ru_maxrss\n",
         "            rss[phase] = usage.ru_maxrss\n"
         "            cpu_seconds[phase] = {'user': usage.ru_utime, 'system': usage.ru_stime}\n"),
        ('"phase_peak_rss_bytes": rss,', '"phase_peak_rss_bytes": rss, "phase_cpu_seconds": cpu_seconds,'),
    )
    for old, new in updates:
        assert source.count(old) == 1, old
        source = source.replace(old, new)
    scope = {'__file__': str(DRIVER), '__name__': 'pinned_deployment_driver'}
    exec(compile(source, str(DRIVER), 'exec'), scope)
    return scope['compile_and_link'], source


def main():
    prior = read(PLAN)['actors']
    control = prior['mech-ascii-miss']
    actors = {CONTROL: control}
    identity = {}
    for role, name in CHANGES.items():
        location = HERE / name / 'build'
        build = read(location / 'result.json')
        exe = location / 'compile_profile'
        assert build['base'] == BASE and build['compiler_sha256'] == sha(exe)
        assert build['artifacts_sha256'] == control['artifact_sha256']
        assert build['tests']['tests_passed'] == 15
        assert build['tests']['tests_log_sha256'] == sha(location / 'tests.log')
        identity[role] = build
        actors[role] = control | {'compile': [str(exe), *control['compile'][1:]], 'compiler_sha256': sha(exe)}
    actors.update({role: prior[role] for role in PEERS})
    for actor in actors.values():
        assert sha(actor['compile'][0]) == actor['compiler_sha256']
        for path, expected in actor['host_inputs_sha256'].items():
            assert sha(path) == expected
    assert all(actors[role]['flags'] == control['flags'] for role in CHANGES)
    cycle = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 2, 4, 1, 3)]
             for shift in range(len(ROLES))]
    schedule = cycle + [list(reversed(row)) for row in cycle]
    run, source = measure_function()
    MATCHED.mkdir()
    (MATCHED / 'timing-driver.py').write_text(source)
    plan = {'source_pin': BASE, 'schedule': schedule, 'actors': actors, 'candidates': identity,
            'clock': 'source through final deployment link',
            'cpu_accounting': 'wait4 user and system time per child phase',
            'timer_sha256': sha(MATCHED / 'timing-driver.py'),
            'parent_timer_sha256': sha(DRIVER), 'load_average_before': os.getloadavg()}
    save(MATCHED / 'plan.json', plan)

    def trial(role, name):
        actor = actors[role]
        return run(role, MATCHED / name / role, actor['compile'], actor['link'], actor['reference_object'],
                   actor['flags'], actor['artifact_sha256'])

    warm = {role: trial(role, 'warmup') for role in ROLES}
    data = []
    load = []
    for i, roles in enumerate(schedule, 1):
        data.append({role: trial(role, f'order-{i}') for role in roles})
        load.append(os.getloadavg())
    mechs = (CONTROL, *CHANGES)
    for row in [warm, *data]:
        assert len({row[role]['stripped_executable']['sha256'] for role in mechs}) == 1
        assert all(row[role]['stripped_executable'] == warm[role]['stripped_executable'] for role in ROLES)
    sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
    link = import_module('recheck-link')
    qualifications = {}
    for role in CHANGES:
        linked = MATCHED / 'worker-checks' / role
        exact = link.link_worker(previous.TEMPLATE, MATCHED / 'order-1' / role / 'frontend', linked)
        assert exact['worker_sha256'] == sha(previous.TEMPLATE / 'mech-worker')
        link.check_worker(linked, 'ssa-comparison-' + role)
        validation = read(linked / 'qualification.json')
        assert validation['complete_smokes'] == 162
        qualifications[role] = {'linked': exact, 'checks': validation}
    medians = {role: {phase: median(row[role]['seconds'][phase] for row in data)
                      for phase in warm[role]['seconds']} for role in ROLES}
    cpu = {role: {phase: median(sum(row[role]['phase_cpu_seconds'][phase].values()) for row in data)
                  for phase in warm[role]['phase_cpu_seconds']} for role in ROLES}
    phases = {}
    for role in mechs:
        values = [{entry['phase']: entry['seconds'] for entry in row[role]['compiler_phases']} for row in data]
        phases[role] = {phase: median(item[phase] for item in values) for phase in values[0]}

    def bracket(role, sibling):
        key = 'source_through_link'
        elapsed = [1000 * (row[sibling]['seconds'][key] - row[role]['seconds'][key]) for row in data]
        ratios = median(row[role]['seconds'][key] / row[sibling]['seconds'][key] for row in data)
        processor = [1000 * (sum(row[sibling]['phase_cpu_seconds']['source'].values()) -
                             sum(row[role]['phase_cpu_seconds']['source'].values())) for row in data]
        return {'paired_elapsed_savings_ms': elapsed, 'elapsed_savings_interval_ms': previous.paired_interval(elapsed),
                'elapsed_x': ratios, 'paired_source_cpu_savings_ms': processor,
                'source_cpu_savings_interval_ms': previous.paired_interval(processor)}

    compared = {role: {peer: bracket(role, peer) for peer in ROLES if peer != role} for role in CHANGES}
    save(MATCHED / 'comparison.json', {'plan_sha256': sha(MATCHED / 'plan.json'), 'median_seconds': medians,
         'median_child_cpu_seconds': cpu, 'phase_medians_seconds': phases, 'paired': compared,
         'qualified_workers': qualifications, 'warmups': warm, 'trials': data, 'load_average_after_blocks': load})
    print({role: {k: v['elapsed_savings_interval_ms'] for k, v in frames.items() if k == CONTROL}
           for role, frames in compared.items()}, cpu, phases, flush=True)


if __name__ == '__main__':
    main()
