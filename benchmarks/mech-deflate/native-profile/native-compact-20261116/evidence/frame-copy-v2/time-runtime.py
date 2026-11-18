"""Screen the 71,192 B exact worker after checking the recovered periodic control."""
from importlib import import_module
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
time = import_module('full-timing')
BOUND = HERE / 'bound'
time.control = BOUND / 'worker-checks/control'
CANDIDATE = BOUND / 'worker-checks/shared-frame-copies'
time.policy_file = time.packet / 'qualification-policy-v2.json'
PERIODIC = [f'prefix-periodic-{n}' for n in (1024, 4096, 8192, 16384)]
time.focus = ['empty', 'seven-stored-prefix', '320-overlap', 'random-64', 'random-1024',
              'prefix-random-1024', 'prefix-natural-8192', 'reymont-prefix-16384',
              'heldout-random-65537', *PERIODIC]


def prepare(kind):
    linked = BOUND / 'linked-proofs/result.json'
    proof = time.read_json(linked)
    assert proof['matches_success_observations'] == 162 and len(proof['checked_failure_cases']) == 6
    assert proof['peak_stack_bytes'] == {'control': 5280, 'shared-frame-copies': 5280}
    info = time.read_json(BOUND / 'binding.json')
    identities = [info['artifacts'][key] for key in ('control', 'shared-frame-copies')]
    assert len({row['worker_sha256'] for row in identities}) == 2
    for row in identities:
        observed = row['linked_encoder']
        assert time.digest(Path(observed['path'])) == observed['sha256'] == row['object_sha256']
        assert row['link'][observed['argument_index']] == f"link-arg={observed['path']}"
    label = f'frame-copy-bound-v2-{kind}'
    folder = HERE / kind
    time.prepare(label, CANDIDATE, folder)
    plan_path = folder / 'plan.json'
    plan = time.read_json(plan_path)
    for key in ('source_commit', 'source_reproduction', 'metadata_reconciliation', 'original_focus',
                'original_repeatability', 'history'):
        plan.pop(key, None)
    contract = folder / 'contract.json'
    contract.unlink()
    plan['identities']['contract_sha256'] = time.put(contract, {
        'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'compress-into invocation through private heap cleanup',
        'fixture_inventory_sha256': time.read_json(time.policy_file)['fixture_inventory_sha256'],
    })
    plan.update(policy_version=2, objective='encoder text bytes',
                source_pin=info['source_pin'], driver=time.ref(Path(__file__).resolve()),
                qualification_driver=time.ref(Path(time.__file__)),
                exact_bindings=time.ref(BOUND / 'binding.json'), linked_proof=time.ref(linked),
                max_peak_stack_bytes=5280,
                runs=[{'id': f'{label}-{order}', 'kind': kind, 'order': order, 'blocks_per_case': 96}
                      for order in ('forward', 'reverse')])
    if kind == 'full':
        plan['focused_prerequisite'] = time.ref(HERE / 'focus/focus-replay.json')
    plan_path.unlink()
    (folder / 'plan.sha256').write_text(time.put(plan_path, plan) + '\n')
    return folder, plan


def acquire(kind):
    folder, plan = prepare(kind)
    time.acquire(plan, CANDIDATE, kind)
    time.replay(plan, folder, kind)
    return time.read_json(folder / f'{kind}-replay.json')


def suitable(report):
    if report['rejected'] or len(report['runs']) != 2:
        return False
    for row in report['runs']:
        order = row['order']
        peer = row['peers']
        control_sentinel = time.read_json(Q / row['id'] / 'summary.json')
        ratios = {x['case']: x['paired_x_libdeflate'] for x in control_sentinel}
        for name in PERIODIC:
            if ratios[name]['mech-control'] >= 1:
                print(order, 'reference periodic sentinel misses parity:', name, flush=True)
                return False
        if any(values['lost_cases'] for values in peer.values()):
            print(order, 'focused peer loss', flush=True)
            return False
        if peer['roc']['point_estimate_wins']['mech-experiment'] != len(time.focus):
            print(order, 'focused Roc floor', flush=True)
            return False
    return True


if __name__ == '__main__':
    focus = acquire('focus')
    if suitable(focus):
        acquire('full')
