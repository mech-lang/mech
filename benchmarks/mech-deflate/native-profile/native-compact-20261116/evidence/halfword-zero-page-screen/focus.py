"""Measure the halfword mapping change against the same encoder and fixed peers."""
from importlib import import_module
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
timing = import_module('full-timing')
study = HERE / 'bound-reference'
timing.control = study / 'worker-checks/control'
candidate = study / 'worker-checks/halfword-mapped'
timing.policy_file = timing.packet / 'qualification-policy-v2.json'
timing.focus = [
    'empty', 'seven-stored-prefix', '320-overlap', 'random-64', 'random-1024',
    'prefix-periodic-1024', 'prefix-periodic-4096', 'prefix-periodic-16384',
    'prefix-random-4096', 'prefix-natural-8192',
]
label = 'halfword-lazy-zeros-focus-v2'
folder = study / 'focus'
plan_file = folder / 'plan.json'


def prepare():
    proof = study / 'linked-proofs/result.json'
    limits = timing.read_json(proof)
    assert limits['matches_success_observations'] == 162 and len(limits['checked_failure_cases']) == 6
    assert limits['peak_stack_bytes']['halfword-mapped'] <= limits['peak_stack_bytes']['control'] == 5280
    control, experiment = [timing.read_json(actor / 'runtime.json') for actor in (timing.control, candidate)]
    for bound in (control, experiment):
        linked = bound['linked_encoder']
        assert bound['link'][linked['argument_index']] == f"link-arg={linked['path']}"
        assert timing.digest(Path(linked['path'])) == linked['sha256'] == bound['object_sha256']
    assert control['worker_sha256'] != experiment['worker_sha256']
    assert control['object_sha256'] == experiment['object_sha256']
    assert control['metadata_sha256'] == experiment['metadata_sha256']

    timing.prepare(label, candidate, folder)
    plan = timing.read_json(plan_file)
    for key in ('source_commit', 'source_reproduction', 'metadata_reconciliation', 'original_focus',
                'original_repeatability', 'history'):
        plan.pop(key, None)
    contract = folder / 'contract.json'
    contract.unlink()
    plan['identities']['contract_sha256'] = timing.put(contract, {
        'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'compress-into invocation through private heap cleanup',
        'fixture_inventory_sha256': timing.read_json(timing.policy_file)['fixture_inventory_sha256'],
    })
    plan.update(policy_version=2, objective='prefix-periodic-1024 elapsed call time',
                hypothesis='Lazy zero pages can reduce ordinary backing work for the large halfword allocation.',
                expected_scope='Focused causal screen of the runtime backing policy with a fixed encoder.',
                driver=timing.ref(Path(__file__).resolve()), qualification_driver=timing.ref(Path(timing.__file__)),
                linked_proof=timing.ref(proof), runtime_patch=timing.ref(HERE / 'zero-pages.patch'),
                max_peak_stack_bytes=5280,
                runs=[{'id': f'{label}-{order}', 'kind': 'focus', 'order': order, 'blocks_per_case': 96}
                      for order in ('forward', 'reverse')])
    assert plan['identities']['mech-control']['worker'] == '674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9'
    plan_file.unlink()
    (folder / 'plan.sha256').write_text(timing.put(plan_file, plan) + '\n')


def main():
    prepare()
    assert timing.digest(plan_file) == (folder / 'plan.sha256').read_text().strip()
    plan = timing.read_json(plan_file)
    for key in ('policy', 'timer', 'driver', 'qualification_driver', 'as_run_inventory',
                'pinned_peers', 'linked_proof', 'runtime_patch'):
        pin = plan[key]
        assert timing.digest(Q / pin['path']) == pin['sha256'], key
    timing.acquire(plan, candidate, 'focus')
    timing.replay(plan, folder, 'focus')


if __name__ == '__main__':
    main()
