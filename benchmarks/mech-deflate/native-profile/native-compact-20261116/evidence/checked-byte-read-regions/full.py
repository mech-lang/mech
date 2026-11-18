"""Run both complete peer orders for the checked byte-read outline."""
from importlib import import_module
from pathlib import Path
import sys

here = Path(__file__).resolve().parent
q = here.parents[1]
sys.path.insert(0, str(q / 'source-frame-copy-tails'))
runtime = import_module('full-timing')
actors = here / 'checks/worker-checks'
runtime.control = actors / 'control'
candidate = actors / 'general-byte-read'
runtime.policy_file = runtime.packet / 'qualification-policy-v2.json'
runtime.focus = [
    'empty', 'one-stored', 'seven-stored-prefix', '320-overlap', 'random-64',
    'random-1024', 'prefix-random-1024', 'prefix-random-4096', 'prefix-random-8192',
    'prefix-natural-4096', 'prefix-natural-8192', 'prefix-natural-16384',
    'prefix-periodic-1024', 'prefix-periodic-4096', 'heldout-mixed-131073',
]
label = 'checked-byte-read-regions-full-v2'
folder = here / 'full'
plan_file = folder / 'plan.json'
plan_sha = folder / 'plan.sha256'


def prepare():
    proof = here / 'checks/linked-proofs/result.json'
    checks = runtime.read_json(proof)
    assert checks['matches_success_observations'] == 162 and len(checks['checked_failure_cases']) == 6
    assert checks['peak_stack_bytes']['general-byte-read'] == checks['peak_stack_bytes']['control'] == 5280
    focus_receipt = here / 'focus/focus-replay.json'
    screen = runtime.read_json(focus_receipt)
    assert not screen['rejected'] and len(screen['runs']) == 2
    assert all(entry['peers']['roc']['point_estimate_wins']['mech-experiment'] == 15 and
               entry['peers']['libdeflate']['point_estimate_wins']['mech-experiment'] == 11
               for entry in screen['runs'])
    runtime.prepare(label, candidate, folder)
    plan = runtime.read_json(plan_file)
    for key in ('source_commit', 'source_reproduction', 'metadata_reconciliation', 'original_focus',
                'original_repeatability', 'history'):
        plan.pop(key, None)
    contract = folder / 'contract.json'
    contract.unlink()
    plan['identities']['contract_sha256'] = runtime.put(contract, {
        'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'compress-into invocation through private heap cleanup',
        'fixture_inventory_sha256': runtime.read_json(runtime.policy_file)['fixture_inventory_sha256'],
    })
    plan.update(policy_version=2, objective='encoder_text_bytes',
                expected_scope='Complete production comparison of the diagnostic encoder workers under policy v2.',
                measured_focus=runtime.ref(focus_receipt),
                review_commit='d1a9ae0c3f4f8945ddd11fd3903edd6aaab089e2',
                driver=runtime.ref(Path(__file__).resolve()), qualification_driver=runtime.ref(Path(runtime.__file__)),
                linked_proof=runtime.ref(proof), source_screen=runtime.ref(here / 'screen/summary.json'),
                max_peak_stack_bytes=5280,
                runs=[{'id': f'{label}-{order}', 'kind': 'full', 'order': order, 'blocks_per_case': 96}
                      for order in ('forward', 'reverse')])
    assert plan['identities']['mech-control']['worker'] == '674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9'
    plan_file.unlink()
    plan_sha.write_text(runtime.put(plan_file, plan) + '\n')
    print(plan_sha.read_text().strip(), flush=True)


def main():
    for task in sys.argv[1:]:
        if task == 'prepare':
            prepare()
            continue
        assert runtime.digest(plan_file) == plan_sha.read_text().strip()
        plan = runtime.read_json(plan_file)
        for key in ('policy', 'timer', 'driver', 'qualification_driver', 'as_run_inventory',
                    'pinned_peers', 'linked_proof', 'source_screen', 'measured_focus'):
            pin = plan[key]
            assert runtime.digest(q / pin['path']) == pin['sha256'], key
        if task == 'acquire':
            runtime.acquire(plan, candidate, 'full')
        elif task == 'replay':
            runtime.replay(plan, folder, 'full')
        else:
            raise ValueError(task)


if __name__ == '__main__':
    raise SystemExit("Superseded: both historical workers linked the reference object. See README.md.")
