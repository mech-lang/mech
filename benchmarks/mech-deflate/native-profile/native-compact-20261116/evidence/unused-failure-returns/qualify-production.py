"""Compare the maintained cleanup path at the matched deployment boundary."""
from argparse import ArgumentParser
from importlib import import_module
from pathlib import Path
from statistics import median
import sys

from build import HERE, Q, sha, write

sys.path.insert(0, str(HERE.parent))
driver = import_module('measure-deployment-builds')
sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
link = import_module('recheck-link')
EXPECTED = HERE.parent / 'statement-compiler/matched/plan.json'
PRODUCTION = HERE / 'production'
FRAMES = HERE.parent / 'frame-copy-v2'
CONTROL, CANDIDATE, ROC, C = 'mech-control', 'mech-early-error-pruning', 'roc', 'libdeflate'
ROLES = (CONTROL, CANDIDATE, ROC, C)


def actor_plan(plan, frame):
    control = plan['variants']['mech-statement-replay']
    compiler = PRODUCTION / 'compile_profile'
    assert sha(compiler) == (PRODUCTION / 'compiler.sha256').read_text().strip() != control['compiler_sha256']
    original, = [path for path in control['link_command'] if path.endswith('/program.o')]
    selected = control['expected_artifacts'].copy()
    flags = control['native_flags']
    proof = production_worker(control, selected, frame)
    if frame:
        frames = driver.read(FRAMES / 'bound/binding.json')
        stamp = frames['artifacts']['shared-frame-copies']
        flags = frames['flags']
        selected.update(encoder=stamp['object_sha256'], metadata=stamp['metadata_sha256'])
    options = {
        CONTROL: (control['source_command'], control['link_command'], original, control['native_flags'],
                  control['expected_artifacts']),
        CANDIDATE: ([str(compiler), *control['source_command'][1:]], control['link_command'],
                    original, flags, selected),
    }
    for actor in (ROC, C):
        recorded = plan['variants'][actor]
        output, = [path for path in recorded['link_command'] if path.endswith(('/encoder.a', '/encoder.o'))]
        options[actor] = (recorded['source_command'], recorded['link_command'], output,
                          recorded['native_flags'], recorded['expected_artifacts'])
    return options, proof


def production_worker(control, original, frame):
    compiler = PRODUCTION / 'compile_profile'
    native = PRODUCTION / 'rebuild/native'
    for name, field in [('program.o', 'encoder'), ('metadata.json', 'metadata')]:
        assert sha(native / name) == original[field]
    assert sha(PRODUCTION / 'rebuild/frontend/program.mecb') == original['bytecode']
    base = FRAMES / 'bound/worker-checks/control'
    out = PRODUCTION / 'worker'
    identities = {'reference': link.link_worker(base, native, out)}
    assert identities['reference']['worker_sha256'] == sha(base / 'mech-worker')
    link.check_worker(out, 'production-omitted-error-returns')
    if frame:
        record = driver.read(FRAMES / 'full/full-replay.json')
        assert record['decision'] == 'Retain' and len(record['runs']) == 2
        for run in record['runs']:
            assert run['peers'][ROC]['point_estimate_wins']['mech-experiment'] == 81
            assert run['peers'][C]['point_estimate_wins']['mech-experiment'] >= 61
    return identities


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('--frame', action='store_true')
    args = parser.parse_args()
    plan = driver.read(EXPECTED)
    choices, identities = actor_plan(plan, args.frame)
    output = PRODUCTION / 'matched'
    output.mkdir()
    order = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 1, 3, 2)]
             for shift in range(len(ROLES))] * 2
    opts = {}
    for role, configuration in choices.items():
        source, provider, obj, flags, pins = configuration
        opts[role] = {'compile': source, 'compiler_sha256': sha(Path(source[0])),
                      'link': provider, 'reference_object': obj, 'flags': flags, 'artifact_sha256': pins,
                      'host_inputs_sha256': {arg: sha(Path(arg)) for arg in provider[:provider.index('-o')]
                                             if arg != obj and arg.endswith(('.o', '.a', '.rlib'))}}
    write(output / 'plan.json', {'previous_plan_sha256': sha(EXPECTED), 'order': order,
                                 'warmup': ROLES, 'frame_copies': args.frame, 'actors': opts,
                                 'reference_worker': identities['reference']['worker_sha256']})

    def sample(role, trial):
        row = driver.compile_and_link(role, output / trial / role, *choices[role])
        assert row['generated_artifacts'] == opts[role]['artifact_sha256']
        return row

    warmups = {role: sample(role, 'warmup') for role in ROLES}
    trials = [{role: sample(role, f'order-{index}') for role in row} for index, row in enumerate(order, 1)]
    for row in trials:
        for role in ROLES:
            assert row[role]['stripped_executable'] == warmups[role]['stripped_executable']
        if not args.frame:
            assert row[CONTROL]['stripped_executable'] == row[CANDIDATE]['stripped_executable']
    stats = {
        'plan_sha256': sha(output / 'plan.json'),
        'medians_seconds': {actor: {field: median(row[actor]['seconds'][field] for row in trials)
                                    for field in warmups[actor]['seconds']} for actor in ROLES},
        'compiler_phase_medians_seconds': {
            role: {phase['phase']: median(next(p['seconds'] for p in row[role]['compiler_phases']
                                                if p['phase'] == phase['phase']) for row in trials)
                   for phase in warmups[role]['compiler_phases']} for role in ROLES[:2]},
        'candidate_paired_build_x': {
            actor: median(row[CANDIDATE]['seconds']['source_through_link'] /
                          row[actor]['seconds']['source_through_link'] for row in trials)
            for actor in (CONTROL, ROC, C)},
        'compiler_peak_rss_bytes': {role: median(row[role]['phase_peak_rss_bytes']['source'] for row in trials)
                                    for role in ROLES},
        'qualified_reference_runtime': identities['reference'],
        'warmups': warmups, 'trials': trials,
    }
    write(output / 'comparison.json', stats)
    print(stats['medians_seconds'], stats['candidate_paired_build_x'], flush=True)


if __name__ == '__main__':
    main()
