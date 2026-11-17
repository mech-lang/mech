"""Build the maintained ASCII scanner and qualify it at the deployment boundary."""
from importlib import import_module
from pathlib import Path
from shutil import copy2
from statistics import median
import os
import re
import subprocess
import sys

from stage import HERE, Q, ROOT, PIN, CHECKOUT, GRAMMAR, read, save, sha

sys.path.insert(0, str(HERE.parent))
driver = import_module('measure-deployment-builds')
sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
link = import_module('recheck-link')

OUT = HERE / 'production'
CURRENT = HERE.parent / 'unused-error-shells/production/matched'
TARGET = HERE.parent / 'definition-compiler/target'
REBUILD = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py'
TEMPLATE = HERE.parent / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
PUBLISHED = 'mech-early-error-pruning'
CONTROL, CANDIDATE = 'mech-control', 'mech-sealed-ascii'
ROLES = (CONTROL, CANDIDATE, 'roc', 'libdeflate')


def build():
    standalone = read(HERE / 'measured/comparison.json')
    assert standalone['pair_ratio']['candidate'] < 0.90
    assert '536 passed; 0 failed' in read(HERE / 'candidate/tests.json')['summary']
    assert subprocess.check_output(['git', 'rev-parse', BRANCH], cwd=ROOT).decode().strip() == PIN
    current = CHECKOUT / GRAMMAR
    original = subprocess.check_output(['git', 'show', f'{PIN}:{GRAMMAR}'], cwd=ROOT)
    assert current.read_bytes() == original
    maintained = HERE / 'candidate/source' / GRAMMAR
    assert maintained.is_file(), maintained
    OUT.mkdir()
    plan = read(CURRENT / 'plan.json')
    selected = plan['actors'][PUBLISHED]
    compiler = OUT / 'compile_profile'
    command = [sys.executable, '-B', REBUILD, OUT / 'rebuild', '--variant', 'frame-copies',
               '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
               '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
    try:
        copy2(maintained, current)
        with (OUT / 'build.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        copy2(TARGET / 'release/compile_profile', compiler)
        for path, key in [('frontend/program.mecb', 'bytecode'), ('native/program.o', 'encoder'),
                          ('native/metadata.json', 'metadata')]:
            assert sha(OUT / 'rebuild' / path) == selected['artifact_sha256'][key], path
    finally:
        current.write_bytes(original)
    assert sha(compiler) != selected['compiler_sha256']
    save(OUT / 'build.json', {'base': PIN, 'source_path': str(GRAMMAR), 'source_sha256': sha(maintained),
         'source_tree_changed_paths': [str(GRAMMAR)], 'compiler_sha256': sha(compiler),
         'compiler_build_command': list(map(str, command)), 'environment': {'CARGO_NET_OFFLINE': 'true'},
         'reference_plan_sha256': sha(CURRENT / 'plan.json'), 'artifact_sha256': selected['artifact_sha256']})
    print(compiler, sha(compiler), flush=True)


def qualify():
    receipt = read(OUT / 'build.json')
    pins = receipt['artifact_sha256']
    rebuilt = OUT / 'rebuild/native'
    identity = link.link_worker(TEMPLATE, rebuilt, OUT / 'worker')
    assert identity['object_sha256'] == pins['encoder']
    assert identity['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
    link.check_worker(OUT / 'worker', 'sealed-ascii')
    save(OUT / 'qualification.json', identity)


def measure():
    pinned = read(CURRENT / 'plan.json')
    receipt = read(OUT / 'build.json')
    assert sha(CURRENT / 'plan.json') == receipt['reference_plan_sha256']
    assert read(OUT / 'worker/qualification.json')['complete_smokes'] == 162
    control = pinned['actors'][PUBLISHED]
    new_executable = str(OUT / 'compile_profile')
    candidate = control | {'compile': [new_executable, *control['compile'][1:]],
                           'compiler_sha256': receipt['compiler_sha256']}
    actors = dict(zip(ROLES, (control, candidate, pinned['actors']['roc'],
                              pinned['actors']['libdeflate']), strict=True))
    for role, actor in actors.items():
        assert sha(Path(actor['compile'][0])) == actor['compiler_sha256'], role
        for path, digest in actor['host_inputs_sha256'].items():
            assert sha(Path(path)) == digest, path
    assert sha(Path(control['compile'][1])) == read(HERE / 'measured/comparison.json')['exact_parse_report']['source_sha256']
    orders = [[ROLES[(i + shift) % len(ROLES)] for i in (0, 1, 3, 2)]
              for shift in range(len(ROLES))] * 2
    dest = OUT / 'matched'
    dest.mkdir()
    save(dest / 'plan.json', {'reference_plan_sha256': receipt['reference_plan_sha256'], 'actors': actors,
         'order': orders, 'warmup': ROLES, 'reference_worker': read(OUT / 'qualification.json'),
         'boundary': 'fresh-process source through final matched deployment link; strip separately'})

    def sample(role, trial):
        actor = actors[role]
        return driver.compile_and_link(role, dest / trial / role, actor['compile'], actor['link'],
                                       actor['reference_object'], actor['flags'], actor['artifact_sha256'])

    warmup = {role: sample(role, 'warmup') for role in ROLES}
    trials = [{role: sample(role, f'order-{index}') for role in order}
              for index, order in enumerate(orders, 1)]
    for row in [warmup, *trials]:
        for role in ROLES:
            assert row[role]['stripped_executable'] == warmup[role]['stripped_executable']
        assert row[CANDIDATE]['stripped_executable'] == row[CONTROL]['stripped_executable']
    medians = {}
    phase_medians = {}
    residuals = {}
    phases = ('parse', 'native', 'frontend', 'catalog')
    for role in ROLES:
        medians[role] = {field: median(row[role]['seconds'][field] for row in trials)
                         for field in warmup[role]['seconds']}
        if role.startswith('mech-'):
            per_trial = [{p['phase']: p['seconds'] for p in row[role]['compiler_phases']} for row in trials]
            phase_medians[role] = {phase: median(row[phase] for row in per_trial) for phase in per_trial[0]}
            residuals[role] = median(row[role]['seconds']['source_through_link'] -
                                     sum(observed[phase] for phase in phases)
                                     for row, observed in zip(trials, per_trial, strict=True))
    ratios = {role: median(row[CANDIDATE]['seconds']['source_through_link'] /
                           row[role]['seconds']['source_through_link'] for row in trials)
              for role in ROLES if role != CANDIDATE}
    result = {'plan_sha256': sha(dest / 'plan.json'), 'medians_seconds': medians,
              'compiler_phase_medians_seconds': phase_medians, 'within_trial_residual_medians_seconds': residuals,
              'candidate_paired_build_x': ratios, 'warmups': warmup, 'trials': trials}
    save(dest / 'comparison.json', result)
    print(medians, phase_medians, residuals, ratios, flush=True)


def compare_parsers():
    matched = OUT / 'matched/plan.json'
    plan = read(matched)
    mech = plan['actors'][CANDIDATE]
    roc = plan['actors']['roc']
    assert '--jobs=1' in roc['compile'] and '--no-cache' in roc['compile']
    actors = {CANDIDATE: mech, 'roc': roc | {'compile': [*roc['compile'][:2], '--timings', *roc['compile'][2:]]}}
    orders = [(CANDIDATE, 'roc'), ('roc', CANDIDATE), ('roc', CANDIDATE), (CANDIDATE, 'roc')]
    dest = OUT / 'parser-phases'
    dest.mkdir()
    save(dest / 'plan.json', {'matched_plan_sha256': sha(matched), 'order': orders, 'actors': actors,
         'source_sha256': {role: sha(next(Path(p) for p in a['compile'] if p.endswith(('.mec', '.roc'))))
                           for role, a in actors.items()},
         'reported_boundaries': {CANDIDATE: 'lossless parse, syntax indexes, strict clean check and cast',
                                 'roc': 'Roc --timings Parsing (summed), one job, no cache'}})

    def run(role, trial):
        actor = actors[role]
        assert sha(Path(actor['compile'][0])) == actor['compiler_sha256']
        folder = dest / trial / role
        result = driver.compile_and_link(role, folder, actor['compile'], actor['link'],
                                         actor['reference_object'], actor['flags'], actor['artifact_sha256'])
        if role == 'roc':
            display = (folder / 'source.log').read_text()
            parsed = re.search(r'^\s*Parsing \(summed\)\s+([\d_.]+)(ns|µs|ms|s)(?:,|\s|$)', display, re.M)
            assert parsed is not None, display[:2000]
            value, unit = parsed.groups()
            result['printed_interval'] = parsed[0].strip()
            result['parser_seconds'] = float(value.replace('_', '')) * {'ns': 1e-9, 'µs': 1e-6, 'ms': 1e-3, 's': 1}[unit]
        else:
            result['parser_seconds'], = [p['seconds'] for p in result['compiler_phases'] if p['phase'] == 'parse']
        assert result['stripped_executable'] == read(OUT / 'matched/warmup' / role / 'result.json')['stripped_executable']
        return result

    warmup = {role: run(role, 'warmup') for role in actors}
    trials = [{role: run(role, f'order-{index}') for role in order}
              for index, order in enumerate(orders, 1)]
    medians = {role: median(row[role]['parser_seconds'] for row in trials) for role in actors}
    save(dest / 'comparison.json', {'plan_sha256': sha(dest / 'plan.json'), 'medians_seconds': medians,
         'mech_over_roc_reported_interval': medians[CANDIDATE] / medians['roc'], 'warmup': warmup, 'trials': trials})
    print(medians, flush=True)


if __name__ == '__main__':
    for step in sys.argv[1:]:
        {'build': build, 'qualify': qualify, 'measure': measure, 'parsers': compare_parsers}[step]()
