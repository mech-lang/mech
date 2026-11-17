"""Run matched final-link builds with the incremental branch closure."""

from importlib import import_module
from pathlib import Path
from statistics import median
import sys

from build import COMPILER, PLAN, PROBE, read, save, sha
from stage import BASE, FLAG, HERE, Q

CONTROL = 'mech-release'
DISABLED = 'mech-probe-control'
CANDIDATE = 'mech-incremental'
MECH = (CONTROL, DISABLED, CANDIDATE)
PEERS = ('roc', 'libdeflate')
TEMPLATE = HERE.parent / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
OUT = HERE / 'matched'


def acquire():
    previous = read(PLAN)
    current = previous['actors']['mech-ascii-miss']
    record = read(PROBE / 'result.json')
    assert record['base'] == BASE and record['plan_sha256'] == sha(PLAN)
    assert record['compiler_sha256'] == sha(COMPILER)
    assert record['disabled_artifacts'] == current['artifact_sha256']
    assert '17 passed; 0 failed' in (HERE / 'tests.log').read_text()
    probe = current | {'compiler_sha256': sha(COMPILER), 'compile': [str(COMPILER), *current['compile'][1:]]}
    actors = {CONTROL: current, DISABLED: probe,
              CANDIDATE: probe | {'flags': probe['flags'] | {FLAG: '1'}},
              **{peer: previous['actors'][peer] for peer in PEERS}}
    for role, info in actors.items():
        assert sha(info['compile'][0]) == info['compiler_sha256'], role
        for path, digest in info['host_inputs_sha256'].items():
            assert sha(path) == digest, path
        assert (FLAG in info['flags']) == (role == CANDIDATE)
    sys.path.insert(0, str(HERE.parent))
    runner = import_module('measure-deployment-builds')
    sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
    link = import_module('recheck-link')
    OUT.mkdir()
    roles = tuple(actors)
    cycles = [[roles[(index + shift) % len(roles)] for index in (0, 2, 4, 1, 3)]
              for shift in range(len(roles))]
    schedule = cycles + [list(reversed(row)) for row in cycles]
    plan = {'source_pin': BASE, 'compiler_case': record, 'actors': actors, 'order': schedule,
            'boundary': 'fresh process through final deployment link. Fixed deployment support is prebuilt.',
            'test_log_sha256': sha(HERE / 'tests.log')}
    save(OUT / 'plan.json', plan)

    def trial(role, label):
        current = actors[role]
        return runner.compile_and_link(role, OUT / label / role, current['compile'], current['link'],
                                       current['reference_object'], current['flags'], current['artifact_sha256'])

    warmup = {role: trial(role, 'warmup') for role in roles}
    trials = [{role: trial(role, f'order-{order}') for role in placement}
              for order, placement in enumerate(schedule, 1)]
    for sample in [warmup, *trials]:
        assert len({sample[r]['stripped_executable']['sha256'] for r in MECH}) == 1
        assert all(sample[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in roles)
    source = OUT / 'order-1' / CANDIDATE / 'frontend'
    assert sha(source / 'program.o') == current['artifact_sha256']['encoder']
    worker = OUT / 'qualified-worker'
    proof = link.link_worker(TEMPLATE, source, worker)
    assert proof['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
    link.check_worker(worker, 'incremental-guard-closure')
    assert read(worker / 'qualification.json')['complete_smokes'] == 162
    save(OUT / 'qualification.json', proof)
    interval = 'source_through_link'
    medians = {role: {stage: median(row[role]['seconds'][stage] for row in trials)
                     for stage in warmup[role]['seconds']} for role in roles}
    paired = {role: median(row[CANDIDATE]['seconds'][interval] / row[role]['seconds'][interval] for row in trials)
              for role in roles if role != CANDIDATE}
    gains = {role: [1000 * (row[role]['seconds'][interval] - row[CANDIDATE]['seconds'][interval]) for row in trials]
             for role in (CONTROL, DISABLED)}
    phases = {}
    for role in MECH:
        samples = [{entry['phase']: entry['seconds'] for entry in row[role]['compiler_phases']} for row in trials]
        phases[role] = {phase: median(row[phase] for row in samples) for phase in samples[0]}
    summary = {'plan_sha256': sha(OUT / 'plan.json'), 'median_seconds': medians, 'paired_ratios': paired,
               'paired_changes_ms': gains, 'median_saved_ms': {role: median(values) for role, values in gains.items()},
               'compiler_phase_medians': phases, 'qualification_sha256': sha(OUT / 'qualification.json'),
               'warmups': warmup, 'trials': trials}
    save(OUT / 'comparison.json', summary)
    print(medians, paired, gains, phases, flush=True)


if __name__ == '__main__':
    acquire()
