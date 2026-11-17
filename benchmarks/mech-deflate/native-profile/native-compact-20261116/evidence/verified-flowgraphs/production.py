"""Build and compare the scoped Cranelift verification reuse change."""
from difflib import unified_diff
from importlib import import_module
import os
from pathlib import Path
from shutil import copy2
from statistics import median
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
ROOT = Q.parent
CHECKOUT = Q / 'native-review-pr/checkout'
PIN = 'f96c1d4fdbc2db1f98c84b561a37132d0a7b5961'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
TARGET = ROADMAP / 'definition-compiler/target'
PRIOR = ROADMAP / 'ascii-graphemes/production/matched'
OUT = HERE / 'production'
REBUILD = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py'
TEMPLATE = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
CODEGEN = Path('vendor/cranelift-codegen/src')
PATHS = (
    CODEGEN / 'context.rs', CODEGEN / 'verifier/mod.rs', CODEGEN / 'unreachable_code.rs',
    CODEGEN / 'remove_constant_phis.rs',
    Path('hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup/mod.rs'),
)
PREVIOUS = 'mech-sealed-ascii'
CONTROL, CANDIDATE = 'mech-control', 'mech-verified-flowgraph'
ROLES = (CONTROL, CANDIDATE, 'roc', 'libdeflate')

sys.path.insert(0, str(ROADMAP))
driver = import_module('measure-deployment-builds')
sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
link = import_module('recheck-link')
sha, read, save = driver.sha, driver.read, driver.save


def pin_patch():
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT)
    assert git('rev-parse', BRANCH).decode().strip() == PIN
    patches = []
    for path in PATHS:
        before = git('show', f'{PIN}:{path}').decode().splitlines(keepends=True)
        current = (CHECKOUT / path).read_text().splitlines(keepends=True)
        assert current != before, path
        patches.extend(unified_diff(before, current, f'a/{path}', f'b/{path}'))
    (HERE / 'change.patch').write_text(''.join(patches))
    return {str(path): sha(CHECKOUT / path) for path in PATHS}


def build():
    sources = pin_patch()
    OUT.mkdir()
    reference = read(PRIOR / 'plan.json')['actors'][PREVIOUS]
    command = [sys.executable, '-B', REBUILD, OUT / 'rebuild', '--variant', 'frame-copies',
               '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
               '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
    with (OUT / 'build.log').open('wb') as log:
        subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    compiler = OUT / 'compile_profile'
    copy2(TARGET / 'release/compile_profile', compiler)
    pins = reference['artifact_sha256']
    for path, key in (('frontend/program.mecb', 'bytecode'), ('native/program.o', 'encoder'),
                      ('native/metadata.json', 'metadata')):
        assert sha(OUT / 'rebuild' / path) == pins[key], path
    assert pin_patch() == sources
    assert sha(compiler) != reference['compiler_sha256']
    save(OUT / 'build.json', {'base': PIN, 'production_source_sha256': sources,
         'patch_sha256': sha(HERE / 'change.patch'), 'compiler_sha256': sha(compiler),
         'build_command': list(map(str, command)), 'artifact_sha256': pins,
         'reference_plan_sha256': sha(PRIOR / 'plan.json')})
    print('compiler', sha(compiler), flush=True)


def qualify():
    receipt = read(OUT / 'build.json')
    identity = link.link_worker(TEMPLATE, OUT / 'rebuild/native', OUT / 'worker')
    assert identity['object_sha256'] == receipt['artifact_sha256']['encoder']
    assert identity['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
    link.check_worker(OUT / 'worker', 'verified-flowgraph')
    print('worker', identity['worker_sha256'], flush=True)


def measure(label='matched'):
    build = read(OUT / 'build.json')
    assert build['reference_plan_sha256'] == sha(PRIOR / 'plan.json')
    checked = read(OUT / 'worker/qualification.json')
    assert checked['verified'] and checked['complete_smokes'] == 162
    pinned = read(PRIOR / 'plan.json')['actors']
    control = pinned[PREVIOUS]
    candidate = control | {'compile': [str(OUT / 'compile_profile'), *control['compile'][1:]],
                           'compiler_sha256': build['compiler_sha256']}
    actors = dict(zip(ROLES, (control, candidate, pinned['roc'], pinned['libdeflate']), strict=True))
    for role, actor in actors.items():
        assert sha(actor['compile'][0]) == actor['compiler_sha256'], role
        for path, digest in actor['host_inputs_sha256'].items():
            assert sha(path) == digest, path
    warmup_order = ROLES if label == 'matched' else tuple(reversed(ROLES))
    orders = [[warmup_order[(i + shift) % len(ROLES)] for i in (0, 1, 3, 2)]
              for shift in range(len(ROLES))] * 2
    dest = OUT / label
    dest.mkdir()
    save(dest / 'plan.json', {'reference_plan_sha256': build['reference_plan_sha256'], 'actors': actors,
         'order': orders, 'warmup': warmup_order, 'worker_sha256': checked['worker_sha256'],
         'boundary': 'fresh-process source through final matched deployment link; strip separately'})

    def sample(role, trial):
        actor = actors[role]
        return driver.compile_and_link(role, dest / trial / role, actor['compile'], actor['link'],
                                       actor['reference_object'], actor['flags'], actor['artifact_sha256'])

    warmup = {role: sample(role, 'warmup') for role in warmup_order}
    trials = [{role: sample(role, f'order-{index}') for role in order}
              for index, order in enumerate(orders, 1)]
    for row in [warmup, *trials]:
        for role in ROLES:
            assert row[role]['stripped_executable'] == warmup[role]['stripped_executable']
        assert row[CANDIDATE]['stripped_executable'] == row[CONTROL]['stripped_executable']
    medians = {role: {field: median(row[role]['seconds'][field] for row in trials)
                      for field in warmup[role]['seconds']} for role in ROLES}
    phases, residual = {}, {}
    for role in (CONTROL, CANDIDATE):
        by_trial = [{p['phase']: p['seconds'] for p in row[role]['compiler_phases']} for row in trials]
        phases[role] = {name: median(row[name] for row in by_trial) for name in by_trial[0]}
        residual[role] = median(row[role]['seconds']['source_through_link'] -
            sum(phase[name] for name in ('parse', 'native', 'frontend', 'catalog'))
            for row, phase in zip(trials, by_trial, strict=True))
    ratios = {role: median(row[CANDIDATE]['seconds']['source_through_link'] /
                           row[role]['seconds']['source_through_link'] for row in trials)
              for role in (CONTROL, 'roc', 'libdeflate')}
    result = {'plan_sha256': sha(dest / 'plan.json'), 'medians_seconds': medians,
              'compiler_phase_medians_seconds': phases, 'within_trial_residual_medians_seconds': residual,
              'candidate_paired_build_x': ratios, 'warmups': warmup, 'trials': trials}
    save(dest / 'comparison.json', result)
    print(medians, phases, residual, ratios, flush=True)


if __name__ == '__main__':
    for step in sys.argv[1:]:
        {'build': build, 'qualify': qualify, 'measure': measure,
         'confirm': lambda: measure('matched-repeat')}[step]()
