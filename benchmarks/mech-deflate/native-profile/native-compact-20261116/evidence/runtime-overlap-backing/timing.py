"""Measure the fixed-encoder backing route and check the historical win set."""

from contextlib import redirect_stdout
from importlib import import_module
from pathlib import Path
import sys

from build import BASE, HERE, Q, TEMPLATE, read, write, sha

sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
time = import_module('full-timing')
time.policy_file = time.packet / 'qualification-policy-v3.json'
time.focus = [
    'empty', 'one-stored', 'seven-stored-prefix', '320-overlap', 'random-64', 'random-1024',
    'prefix-random-1024', 'prefix-periodic-1024', 'prefix-periodic-4096', 'prefix-natural-4096',
    'prefix-natural-8192', 'prefix-natural-16384', 'prefix-natural-32769', 'prefix-random-4096',
    'heldout-mixed-131073',
]
ROUTE = 'word-pages-32'
TIMING = HERE / 'timing'
FROZEN = 'evidence/runtime-62-high-water/summary.json'
NEAR = ('320-overlap', 'prefix-natural-4096', 'prefix-natural-8192',
        'prefix-periodic-1024', 'prefix-random-1024')


def plan_run(kind):
    candidate = HERE / 'checks/worker-checks' / ROUTE
    if kind == 'focus':
        time.control = HERE / 'checks/worker-checks/control'
        assert sha(time.control / 'mech-worker') == sha(TEMPLATE / 'mech-worker')
        objective = 'Measure a backing change with the emitted encoder held fixed.'
    else:
        screened = read(TIMING / 'focus/screen.json')
        assert screened['preserves_peer_wins'] and screened['both_targets_win']
        time.control = HERE.parent / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
        objective = 'Verify both full peer win floors for the bottom-tested scorer and revised word backing.'
    folder = TIMING / kind
    label = f'overlap-margin-{kind}'
    time.prepare(label, candidate, folder)
    plan = read(folder / 'plan.json')
    for key in ('source_commit', 'review_commit', 'source_reproduction',
                'metadata_reconciliation', 'original_focus', 'original_repeatability', 'history'):
        del plan[key]
    contract = folder / 'contract.json'
    write(contract, {'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'complete compression through private heap cleanup',
        'fixture_inventory_sha256': read(time.policy_file)['fixture_inventory_sha256']})
    plan['identities']['contract_sha256'] = sha(contract)
    plan.update(source_pin=BASE, policy_version=3, objective=objective, max_peak_stack_bytes=5280,
        runs=[{'id': f'{label}-{order}', 'kind': kind, 'order': order, 'blocks_per_case': 96}
              for order in ('forward', 'reverse')],
        driver=time.ref(Path(__file__).resolve()), controller=time.ref(Path(time.__file__)),
        linked_contract=time.ref(HERE / 'checks/linked-proofs/result.json'))
    write(folder / 'plan.json', plan)
    (folder / 'plan.sha256').write_text(sha(folder / 'plan.json') + '\n')
    return folder, candidate, plan


def summarize(folder, kind):
    replay = read(folder / f'{kind}-replay.json')
    history = read(time.packet / FROZEN)
    rows = []
    for run in replay['runs']:
        order, key = run['order'], run['id']
        summary = {r['case']: r for r in read(Q / key / 'summary.json')}
        peer = {name: row['paired_x_libdeflate'] for name, row in summary.items() if name in NEAR}
        wins = {k: r['point_estimate_wins'] for k, r in run['peers'].items()}
        losses = {k: r['lost_cases'] for k, r in run['peers'].items()}
        old = set(history['orders'][order]['peers']['libdeflate']['historical_wins'])
        old &= summary.keys()
        missed = sorted(name for name in old if summary[name]['paired_x_libdeflate']['mech-experiment'] >= 1)
        rows.append({'order': order, 'focus_ratios_vs_libdeflate': peer, 'missed_historical_wins': missed,
                     'wins': wins, 'losses': losses, 'receipts': key})
    result = {'kind': kind, 'policy_version': 3, 'decision': replay['decision'],
              'preserves_peer_wins': not any(k for row in rows for k in row['losses'].values()),
              'both_targets_win': all(row['focus_ratios_vs_libdeflate'][name]['mech-experiment'] < 1
                                    for name in ('320-overlap', 'prefix-natural-8192') for row in rows),
              'runs': rows, 'rejected': replay['rejected'], 'unresolved': replay['unresolved']}
    write(folder / 'screen.json', result)
    for row in rows:
        print(row['order'], row['wins'], [(k, round(v['mech-experiment'], 6))
              for k, v in row['focus_ratios_vs_libdeflate'].items()],
              'lost', row['losses'], 'misses', row['missed_historical_wins'], flush=True)
    print(result['decision'], flush=True)


if __name__ == '__main__':
    scope, = sys.argv[1:]
    assert scope in ('focus', 'full')
    folder, candidate, plan = plan_run(scope)
    with (folder / 'acquire.log').open('w') as log, redirect_stdout(log):
        time.acquire(plan, candidate, scope)
    with (folder / 'replay.log').open('w') as log, redirect_stdout(log):
        time.replay(plan, folder, scope)
    summarize(folder, scope)
