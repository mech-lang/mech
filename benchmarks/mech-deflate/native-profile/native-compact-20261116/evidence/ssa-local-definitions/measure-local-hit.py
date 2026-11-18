"""Measure the cached SSA shortcut from source through deployment link."""

from importlib import import_module
from pathlib import Path
from random import Random
import sys
from statistics import median

found = import_module('build-local-hit')
BASE, HERE, Q, OUT = found.BASE, found.HERE, found.Q, found.OUT
read, save, sha = found.read, found.save, found.sha
PLAN = found.PUBLISHED
MATCHED = HERE / 'local-hit/matched'
CONTROL = 'mech-release'
CANDIDATE = 'mech-ssa-local'
PEERS = ('roc', 'libdeflate')
ACTORS = (CONTROL, CANDIDATE, *PEERS)
TEMPLATE = HERE.parent / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
BOUNDARY = 'source_through_link'


def paired_interval(values):
    random = Random(1916314301)
    count = len(values)
    samples = sorted(median(values[random.randrange(count)] for _ in values) for _ in range(8000))
    return dict(median=median(values), lower_90=samples[len(samples) // 20],
                upper_90=samples[len(samples) * 19 // 20])


def main():
    build = read(OUT / 'result.json')
    prior = read(PLAN)['actors']
    current = prior['mech-ascii-miss']
    compiler = OUT / 'compile_profile'
    assert build['base'] == BASE and build['build_plan_sha256'] == sha(PLAN)
    assert build['compiler_sha256'] == sha(compiler)
    assert build['artifacts_sha256'] == current['artifact_sha256']
    assert build['tests']['tests_passed'] == 15
    assert build['tests']['tests_log_sha256'] == sha(OUT / 'tests.log')
    actors = {
        CONTROL: current,
        CANDIDATE: current | {'compiler_sha256': sha(compiler), 'compile': [str(compiler), *current['compile'][1:]]},
        **{role: prior[role] for role in PEERS},
    }
    for role, actor in actors.items():
        assert sha(Path(actor['compile'][0])) == actor['compiler_sha256'], role
        for name, digest in actor['host_inputs_sha256'].items():
            assert sha(Path(name)) == digest, name
    assert actors[CONTROL]['flags'] == actors[CANDIDATE]['flags']
    cycles = [[ACTORS[(i + shift) % 4] for i in (0, 1, 3, 2)] for shift in range(4)]
    schedule = cycles + [list(reversed(row)) for row in cycles]
    sys.path.insert(0, str(HERE.parent))
    run = import_module('measure-deployment-builds')
    sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
    check = import_module('recheck-link')
    MATCHED.mkdir()
    plan = {'source_pin': BASE, 'candidate_build': build, 'schedule': schedule,
            'deployment_boundary': 'Fresh source through final link with prebuilt fixed deployment support.',
            'actors': actors}
    save(MATCHED / 'plan.json', plan)

    def trial(role, label):
        actor = actors[role]
        return run.compile_and_link(role, MATCHED / label / role, actor['compile'], actor['link'],
                                    actor['reference_object'], actor['flags'], actor['artifact_sha256'])

    warmup = {role: trial(role, 'warmup') for role in actors}
    trials = [{role: trial(role, 'order-' + str(i)) for role in order} for i, order in enumerate(schedule, 1)]
    for row in [warmup, *trials]:
        assert row[CONTROL]['stripped_executable'] == row[CANDIDATE]['stripped_executable']
        assert all(row[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in actors)
    matched_worker = MATCHED / 'linked-worker'
    binding = check.link_worker(TEMPLATE, MATCHED / 'order-1' / CANDIDATE / 'frontend', matched_worker)
    assert binding['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
    check.check_worker(matched_worker, 'ssa-local-hit')
    smoke = read(matched_worker / 'qualification.json')
    assert smoke['complete_smokes'] == 162
    medians = {role: {phase: median(row[role]['seconds'][phase] for row in trials)
                      for phase in warmup[role]['seconds']} for role in ACTORS}
    elapsed = {role: [row[role]['seconds'][BOUNDARY] for row in trials] for role in ACTORS}
    paired = {role: median(x / y for x, y in zip(elapsed[CANDIDATE], elapsed[role], strict=True))
              for role in ACTORS if role != CANDIDATE}
    changes = [1000 * (baseline - shortcut) for baseline, shortcut in zip(elapsed[CONTROL], elapsed[CANDIDATE], strict=True)]
    phases = {}
    for role in (CONTROL, CANDIDATE):
        values = [{entry['phase']: entry['seconds'] for entry in row[role]['compiler_phases']} for row in trials]
        phases[role] = {k: median(r[k] for r in values) for k in values[0]}
    result = {'plan_sha256': sha(MATCHED / 'plan.json'), 'warmups': warmup, 'trials': trials,
              'median_seconds': medians, 'phase_medians_seconds': phases,
              'paired_ratios': paired, 'paired_gains_ms': changes, 'gain_interval_ms': paired_interval(changes),
              'exact_worker': binding, 'qualification': smoke}
    save(MATCHED / 'comparison.json', result)
    print(result['gain_interval_ms'], paired, phases, flush=True)


if __name__ == '__main__':
    main()
