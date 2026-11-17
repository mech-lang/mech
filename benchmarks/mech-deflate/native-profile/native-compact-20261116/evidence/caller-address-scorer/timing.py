"""Run matched scorer screens and full v2 qualification for eligible workers."""
from contextlib import redirect_stdout
from importlib import import_module
from pathlib import Path
import sys

from emit import HERE, PIN, Q, TEMPLATE, read, save, sha

sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
time = import_module('full-timing')
time.control = TEMPLATE
time.policy_file = time.packet / 'qualification-policy-v2.json'
time.focus = [
    'empty', 'one-stored', 'seven-stored-prefix', '320-overlap',
    'random-64', 'random-1024', 'prefix-random-1024', 'prefix-periodic-1024',
    'prefix-natural-4096', 'prefix-natural-8192', 'prefix-natural-16384', 'prefix-natural-32769',
    'reymont-prefix-32769', 'prefix-random-4096', 'heldout-mixed-131073',
]


def paths(label, kind):
    return HERE / label / kind, HERE / label / 'bound'


def setup(label, kind):
    folder, worker = paths(label, kind)
    result = read(worker.parent / 'receipt.json')
    assert result['source_pin'] == PIN
    worker_record = read(worker / 'runtime.json')
    assert result['worker_sha256'] == sha(worker / 'mech-worker') == worker_record['worker_sha256']
    assert worker_record['object_sha256'] == sha(worker / 'program.o') == result['layout']['object_sha256']
    assert result['exact_checks'] == 162 and read(worker / 'qualification.json')['complete_smokes'] == 162
    assert sha(TEMPLATE / 'mech-worker') == 'ddefe57e371bb6f072e6e2a3b2cd3a0d20bf108f7285137d3467304b4a84ed0a'
    stem = 'scorer-20261117-' + label
    time.prepare(stem, worker, folder)
    plan = read(folder / 'plan.json')
    for key in ('source_commit', 'review_commit', 'source_reproduction', 'metadata_reconciliation',
                'original_focus', 'original_repeatability', 'history'):
        plan.pop(key, None)
    contract = folder / 'contract.json'
    contract.unlink()
    plan['identities']['contract_sha256'] = time.put(contract, {
        'scope': 'diagnostic', 'level': 12, 'timing_boundary': 'compress-into through private heap cleanup',
        'fixture_inventory_sha256': read(time.policy_file)['fixture_inventory_sha256'],
    })
    plan.update(policy_version=2, source_pin=PIN,
                objective='Recover the 8 KiB natural-text libdeflate win and shrink encoder text.',
                driver=time.ref(Path(__file__).resolve()), controller=time.ref(Path(time.__file__)),
                max_peak_stack_bytes=5280, candidate_source_build=time.ref(worker.parent / 'receipt.json'),
                runs=[{'id': f'{stem}-{kind}-{order}', 'kind': kind, 'order': order, 'blocks_per_case': 96}
                      for order in ('forward', 'reverse')])
    if kind == 'full':
        plan['focused_prerequisite'] = time.ref(worker.parent / 'focus/screen.json')
        plan['linked_contract'] = time.ref(worker.parent / 'checks/linked-proofs/result.json')
    (folder / 'plan.json').unlink()
    digest = time.put(folder / 'plan.json', plan)
    (folder / 'plan.sha256').write_text(digest + '\n')
    return plan


def summarize(folder, kind):
    report = read(folder / f'{kind}-replay.json')
    rows = []
    for run in report['runs']:
        observed = {item['case']: item for item in read(Q / run['id'] / 'summary.json')}
        cases = {}
        for name in ('prefix-natural-8192', '320-overlap', 'random-1024', 'prefix-periodic-1024'):
            value = observed[name]
            cases[name] = {role: value['paired_x_libdeflate'][role]
                           for role in ('mech-control', 'mech-experiment')}
        rows.append({'order': run['order'], 'cases': cases,
                      'lost_cases': {peer: value['lost_cases'] for peer, value in run['peers'].items()},
                      'gained_cases': {peer: value['gained_cases'] for peer, value in run['peers'].items()},
                      'wins': {peer: value['point_estimate_wins'] for peer, value in run['peers'].items()}})
    record = {'kind': kind, 'policy_decision': report['decision'], 'runs': rows,
              'rejected': report['rejected'], 'unresolved': report['unresolved'],
              'achieves_target': all(row['cases']['prefix-natural-8192']['mech-experiment'] < 1 for row in rows),
              'preserves_peer_wins': len(rows) == 2 and all(not any(row['lost_cases'].values()) for row in rows)}
    save(folder / 'screen.json', record)
    print(folder.parent.name, kind, rows, flush=True)


def main():
    label, kind = sys.argv[1:]
    assert kind in ('focus', 'full')
    folder, worker = paths(label, kind)
    plan = setup(label, kind)
    with (folder / 'acquire.log').open('w') as log, redirect_stdout(log):
        time.acquire(plan, worker, kind)
    with (folder / 'replay.log').open('w') as log, redirect_stdout(log):
        time.replay(plan, folder, kind)
    summarize(folder, kind)


if __name__ == '__main__':
    main()
