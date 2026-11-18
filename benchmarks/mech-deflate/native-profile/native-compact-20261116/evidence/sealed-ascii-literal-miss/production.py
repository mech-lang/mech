"""Qualify a compiled parser shortcut and time the exact deployment artifacts."""

import json
import os
from importlib import import_module
from pathlib import Path
from shutil import copy2
from statistics import median
import subprocess
import sys

from stage import BASE, CHECKOUT, HERE, OUT, Q, ROOT, sha
from measure import RESULTS, read, save

ROADMAP = HERE.parent
PREVIOUS = ROADMAP / 'flowgraph-verification/production/matched-repeat/plan.json'
TEMPLATE = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
TARGET = ROADMAP / 'definition-compiler/target'
PRODUCTION = HERE / 'production'
PREEXISTING = 'mech-verified-flowgraph'
CONTROL = 'mech-control'
CANDIDATE = 'mech-ascii-miss'
PEERS = ('roc', 'libdeflate')
ROLES = (CONTROL, CANDIDATE, *PEERS)

sys.path.insert(0, str(ROADMAP))
driver = import_module('measure-deployment-builds')
sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
link = import_module('recheck-link')


def build():
    standalone = read(RESULTS / 'result.json')
    assert standalone['candidate_x'] < 1, standalone
    assert standalone['plan_sha256'] == sha(RESULTS / 'plan.json')
    assert '0 failed' in read(OUT / 'tests.json')['summary']
    assert subprocess.check_output(
        ['git', 'rev-parse', 'refs/heads/codex/native-compact-source-handoff'],
        cwd=ROOT).decode().strip() == BASE
    pins = read(HERE / 'source.json')
    assert pins['base'] == BASE
    replacements = {}
    for path, wanted in pins['source_sha256'].items():
        local = CHECKOUT / path
        clean = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT)
        candidate = OUT / 'source' / path
        assert local.read_bytes() == clean and sha(candidate) == wanted, path
        replacements[local] = (clean, candidate)
    PRODUCTION.mkdir()
    command = [sys.executable, '-B', PACKET / 'rebuild.py', PRODUCTION / 'rebuild',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        for local, (_, source) in replacements.items():
            copy2(source, local)
        with (PRODUCTION / 'build.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        compiler = PRODUCTION / 'compile_profile'
        copy2(TARGET / 'release/compile_profile', compiler)
        expected = read(PREVIOUS)['actors'][PREEXISTING]['artifact_sha256']
        for file, field in (('frontend/program.mecb', 'bytecode'), ('native/program.o', 'encoder'),
                            ('native/metadata.json', 'metadata')):
            assert sha(PRODUCTION / 'rebuild' / file) == expected[field], field
    finally:
        for local, (clean, source) in replacements.items():
            assert local.read_bytes() in (clean, source.read_bytes()), local
            local.write_bytes(clean)
    save(PRODUCTION / 'build.json', pins | {'command': list(map(str, command)),
         'compiler_sha256': sha(compiler), 'reference_plan_sha256': sha(PREVIOUS),
         'artifact_sha256': expected, 'parser_local_measurement_sha256': sha(RESULTS / 'result.json')})
    print('Production compiler reproduced the three published artifacts', sha(compiler), flush=True)


def qualify():
    receipt = read(PRODUCTION / 'build.json')
    worker = PRODUCTION / 'worker'
    identity = link.link_worker(TEMPLATE, PRODUCTION / 'rebuild/native', worker)
    assert identity['object_sha256'] == receipt['artifact_sha256']['encoder']
    assert identity['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
    link.check_worker(worker, 'sealed-ascii-literal-miss')
    save(PRODUCTION / 'qualification.json', identity)
    print('Reproduced original worker and 162 observations', identity['worker_sha256'], flush=True)


def measure():
    pinned = read(PREVIOUS)
    receipt = read(PRODUCTION / 'build.json')
    assert sha(PREVIOUS) == receipt['reference_plan_sha256']
    qualification = read(PRODUCTION / 'worker/qualification.json')
    assert qualification['complete_smokes'] == 162
    control = pinned['actors'][PREEXISTING]
    compiler = PRODUCTION / 'compile_profile'
    actors = {
        CONTROL: control,
        CANDIDATE: control | {'compile': [str(compiler), *control['compile'][1:]],
                              'compiler_sha256': receipt['compiler_sha256']},
        **{role: pinned['actors'][role] for role in PEERS},
    }
    for role, actor in actors.items():
        assert sha(Path(actor['compile'][0])) == actor['compiler_sha256'], role
        for path, digest in actor['host_inputs_sha256'].items():
            assert sha(Path(path)) == digest, path
    orders = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 1, 3, 2)]
              for shift in range(len(ROLES))] * 2
    dest = PRODUCTION / 'matched'
    dest.mkdir()
    save(dest / 'plan.json', {'reference_plan_sha256': sha(PREVIOUS), 'actors': actors, 'order': orders,
        'worker_sha256': qualification['worker_sha256'],
        'boundary': 'fresh process, source through final link with pinned deployment host and strip separately'})

    def sample(role, label):
        actor = actors[role]
        return driver.compile_and_link(role, dest / label / role, actor['compile'], actor['link'],
                                       actor['reference_object'], actor['flags'], actor['artifact_sha256'])

    warmup = {role: sample(role, 'warmup') for role in ROLES}
    trials = [{role: sample(role, f'order-{ordinal}') for role in order}
              for ordinal, order in enumerate(orders, 1)]
    for row in [warmup, *trials]:
        for role in ROLES:
            assert row[role]['stripped_executable'] == warmup[role]['stripped_executable']
        assert row[CONTROL]['stripped_executable'] == row[CANDIDATE]['stripped_executable']
    medians = {role: {field: median(row[role]['seconds'][field] for row in trials)
                      for field in warmup[role]['seconds']} for role in ROLES}
    phases = {}
    for role in (CONTROL, CANDIDATE):
        grouped = [{phase['phase']: phase['seconds'] for phase in row[role]['compiler_phases']} for row in trials]
        phases[role] = {phase: median(row[phase] for row in grouped) for phase in grouped[0]}
    interval = 'source_through_link'
    ratios = {role: median(row[CANDIDATE]['seconds'][interval] / row[role]['seconds'][interval] for row in trials)
              for role in (CONTROL, *PEERS)}
    gains = [1000 * (row[CONTROL]['seconds'][interval] - row[CANDIDATE]['seconds'][interval]) for row in trials]
    result = {'plan_sha256': sha(dest / 'plan.json'), 'median_seconds': medians,
              'compiler_phases': phases, 'paired_ratios': ratios, 'median_save_ms': median(gains),
              'paired_gains_ms': gains, 'warmup': warmup, 'trials': trials}
    save(dest / 'comparison.json', result)
    print('Build medians', {name: row[interval] for name, row in medians.items()}, flush=True)
    print('Paired ratios', ratios, 'median milliseconds saved', median(gains), flush=True)
    print('Compiler phase medians', phases, flush=True)


if __name__ == '__main__':
    for action in sys.argv[1:]:
        {'build': build, 'qualify': qualify, 'measure': measure}[action]()
