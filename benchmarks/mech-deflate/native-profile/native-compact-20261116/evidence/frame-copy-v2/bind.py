"""Prove today's source compiler emits the archived frame-copy and reference workers."""
from importlib import import_module
import json
import os
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = HERE.parent
sys.path.insert(0, str(ROOT / 'checked-root-census'))
sys.path.insert(0, str(ROOT))
link = import_module('recheck-link')
measured = import_module('measure-deployment-builds')
census = sys.path.insert(0, str(ROOT / 'function-compiler'))
census = import_module('root-census')
SAME = ROOT / 'statement-compiler'
TEMPLATE = ROOT / 'checked-root-census/linked-recheck/worker-checks/control'
HISTORICAL = Q / 'source-frame-copy-tails/verified/shared-copies'
CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
PLAN = SAME / 'matched/plan.json'
HEAD = 'e8b2bb4f2d8c44390e9a67dfe054c6ff40341249'
ROLES = ('control', 'shared-frame-copies')


def main():
    pinned = link.read(PLAN)
    variant = pinned['variants']['mech-statement-replay']
    compiler = Path(variant['source_command'][0])
    assert link.sha(compiler) == variant['compiler_sha256']
    source = Path(variant['source_command'][1])
    assert link.sha(source) == pinned['source_sha256']['mech']
    flags = dict(variant['native_flags'])
    baseline_profile = flags['MECH_COMPACT_ROOT_TAIL_PROFILE']
    recipe = link.read(PACKET / 'recipe.json')
    assert baseline_profile == str(PACKET / recipe['zero_branches_tail_profile'])
    flags |= {'MECH_COMPACT_SHARE_FRAME_COPIES': '1',
              'MECH_COMPACT_ROOT_TAIL_PROFILE': str(PACKET / recipe['frame_copies_tail_profile'])}
    assert flags.keys() - variant['native_flags'].keys() == {'MECH_COMPACT_SHARE_FRAME_COPIES'}
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    out = HERE / 'bound'
    out.mkdir()
    compiled = out / 'compiled'
    command = [str(compiler), str(source), str(compiled), 'compact']
    with (out / 'compile.log').open('wb') as log:
        subprocess.run(command, env=env | flags | {'PATH': '/usr/bin:/bin'},
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    assert link.sha(compiled / 'program.mecb') == variant['expected_artifacts']['bytecode']
    assert link.sha(compiled / 'program.o') == link.sha(HISTORICAL / 'program.o')
    assert link.sha(compiled / 'metadata.json') == link.sha(HISTORICAL / 'metadata.json')
    artifacts = (TEMPLATE, compiled)
    receipts = {role: link.link_worker(TEMPLATE, obj, out / 'worker-checks' / role)
                for role, obj in zip(ROLES, artifacts, strict=True)}
    assert receipts[ROLES[0]]['worker_sha256'] == link.sha(TEMPLATE / 'mech-worker')
    assert receipts[ROLES[1]]['worker_sha256'] == link.sha(HISTORICAL / 'mech-worker')
    assert len({v['worker_sha256'] for v in receipts.values()}) == 2
    for role in ROLES:
        work = out / 'worker-checks' / role
        link.check_worker(work, f'frame-copy-v2-bound-{role}')
    static = {role: census.census(out / 'worker-checks' / role / 'program.o') for role in ROLES}
    assert tuple(static[role]['encoder_text_bytes'] for role in ROLES) == (71768, 71192)
    link.write(out / 'binding.json', {'source_pin': HEAD, 'plan_sha256': link.sha(PLAN), 'emit': command,
                                    'flags': flags, 'artifacts': receipts, 'symbol_sizes': static})
    return link.check_contracts(out, ROLES)


if __name__ == '__main__':
    raise SystemExit(main())
