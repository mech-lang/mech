"""Repeat both complete orders for the exact bottom-tested scorer and checked root."""

from contextlib import redirect_stdout
import os
from pathlib import Path
from sys import argv

from timing import BASE, HERE, Q, TEMPLATE, read, sha, summarize, time, write

OUT = HERE / 'root-confirmation'


def main():
    assert argv[1:] == []
    reference = HERE.parent / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
    control = time.control = reference
    fixed = read(control / 'runtime.json')
    probe = read(TEMPLATE / 'runtime.json')
    assert sha(control / 'mech-worker') == fixed['worker_sha256']
    assert sha(TEMPLATE / 'mech-worker') == probe['worker_sha256']
    qualification_root = HERE.parent / 'scorer-vector-latch/latch-kept-root-layout'
    before = read(qualification_root / 'full/screen.json')
    assert not all(row['wins_mech_experiment']['libdeflate'] == 62 for row in before['runs'])
    proof = qualification_root / 'checks/linked-proofs/result.json'
    verified = read(proof)
    assert verified['matches_success_observations'] == 162
    assert set(verified['peak_stack_bytes'].values()) == {5280}
    label = 'overlap-root-confirmation-full'
    time.prepare(label, TEMPLATE, OUT)
    plan = read(OUT / 'plan.json')
    for key in ('source_commit', 'review_commit', 'source_reproduction',
                'metadata_reconciliation', 'original_focus', 'original_repeatability', 'history'):
        del plan[key]
    contract = OUT / 'contract.json'
    write(contract, {'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'complete compression through private heap cleanup',
        'fixture_inventory_sha256': read(time.policy_file)['fixture_inventory_sha256']})
    plan['identities']['contract_sha256'] = sha(contract)
    plan.update(source_pin=BASE, policy_version=3,
                objective='Verify complete reproducibility of the exact bottom-tested scorer.',
                linked_contract=time.ref(proof), driver=time.ref(Path(__file__).resolve()),
                controller=time.ref(Path(time.__file__)), max_peak_stack_bytes=5280,
                earlier_full_result=time.ref(qualification_root / 'full/screen.json'),
                load_average_before=os.getloadavg(),
                runs=[{'id': f'{label}-{order}', 'kind': 'full', 'order': order, 'blocks_per_case': 96}
                      for order in ('forward', 'reverse')])
    write(OUT / 'plan.json', plan)
    (OUT / 'plan.sha256').write_text(sha(OUT / 'plan.json') + '\n')
    with (OUT / 'acquire.log').open('w') as log, redirect_stdout(log):
        time.acquire(plan, TEMPLATE, 'full')
    write(OUT / 'post-acquisition.json', {'load_average': os.getloadavg()})
    with (OUT / 'replay.log').open('w') as log, redirect_stdout(log):
        time.replay(plan, OUT, 'full')
    summarize(OUT, 'full')


if __name__ == '__main__':
    main()
