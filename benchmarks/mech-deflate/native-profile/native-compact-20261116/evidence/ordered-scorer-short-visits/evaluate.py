"""Rebuild and screen short-visit emissions with the frozen peer workers."""

from contextlib import redirect_stdout
from importlib import import_module
import os
from pathlib import Path
import runpy
import sys

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
build = runpy.run_path(str(HERE / 'build.py'), run_name='short_visit_build')
BASE, COMPILER, LATCH, OUTPUT, PLAN, Q = (build[key] for key in ('BASE', 'COMPILER', 'LATCH', 'OUTPUT', 'PLAN', 'Q'))
read, write, sha = (build[key] for key in ('read', 'write', 'sha'))

sys.path.insert(0, str(ROADMAP / 'ordered-scorer'))
study = import_module('study')
driver = study.driver
driver.HERE = HERE
driver.PIN = BASE
driver.PLAN = HERE / 'control-plan.json'
study.HERE = HERE
for module in (study.rebase, study.scheduler, study.count, study.fit):
    module.HERE = HERE

CONTROL = 'short-visit-latch-control'
QUALIFIED = driver.TEMPLATE
MODE = {
    'vector-4': {'MECH_ORDERED_VECTOR_MINIMUM': '4'},
    'vector-8': {'MECH_ORDERED_VECTOR_MINIMUM': '8'},
    'vector-12': {'MECH_ORDERED_VECTOR_MINIMUM': '12'},
    'paired-scalar': {'MECH_ORDERED_PAIRED_CONTINUATION': '1'},
    'vector-8-paired': {'MECH_ORDERED_VECTOR_MINIMUM': '8', 'MECH_ORDERED_PAIRED_CONTINUATION': '1'},
}
BASELINE_MODE = driver.HORIZONTAL | driver.REVERSE | {'MECH_ORDERED_VECTOR_LATCH_TEST': '1'}


def pin_inputs():
    proved = read(OUTPUT / 'result.json')
    assert proved['source_pin'] == BASE and proved['compiler_sha256'] == sha(COMPILER)
    matched = read(PLAN)
    matched['actors']['mech-verified-flowgraph'] = matched['actors']['mech-ssa-first-definition']
    if driver.PLAN.exists():
        assert read(driver.PLAN) == matched
    else:
        write(driver.PLAN, matched)


def original_layout():
    return ROADMAP / 'scorer-vector-latch/latch-test-scorer-profile.json'


def emit_roles(roles):
    pin_inputs()
    control = HERE / CONTROL / 'receipt.json'
    if not control.exists():
        driver.emit(CONTROL, BASELINE_MODE, COMPILER, original_layout())
    baseline = read(control)
    initial = read(LATCH / 'receipt.json')
    assert baseline['layout']['object_sha256'] == initial['layout']['object_sha256']
    assert baseline['worker_sha256'] == initial['worker_sha256']
    assert baseline['metadata_sha256'] == initial['metadata_sha256']
    if not (HERE / CONTROL / 'instruction-counts/counts.json').exists():
        study.measure_counts(CONTROL)
    for role in roles:
        flags = BASELINE_MODE | MODE[role]
        if not (HERE / role / 'receipt.json').exists():
            driver.emit(role, flags, COMPILER)
        if not (HERE / role / 'instruction-counts/counts.json').exists():
            study.measure_counts(role)
        path = study.profiles(role)
        driver.emit(role + '-layout', flags, COMPILER, path)
        candidate = read(HERE / (role + '-layout') / 'receipt.json')
        fixed = '_mech_compact_closed_checked'
        keys = ('sha256', 'extent', 'relocations')
        assert all(candidate['layout']['symbols'][fixed][key] == baseline['layout']['symbols'][fixed][key]
                   for key in keys)


def check_roles(roles):
    checked = HERE / 'linked'
    views = checked / 'worker-checks'
    groups = {'control': HERE / CONTROL / 'bound',
              **{role: HERE / (role + '-layout') / 'bound' for role in roles}}
    for role, source in groups.items():
        binding = driver.link.link_worker(source, source, views / role)
        assert binding['worker_sha256'] == sha(source / 'mech-worker')
        driver.link.check_worker(views / role, f'short-visit-{role}')
    assert driver.link.check_contracts(checked, tuple(groups)) == 0
    report = read(checked / 'linked-proofs/result.json')
    assert report['matches_success_observations'] == 162
    assert len(report['checked_failure_cases']) == 6
    print(report['peak_stack_bytes'], flush=True)


def runtime_modules():
    sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
    time = import_module('full-timing')
    time.policy_file = time.packet / 'qualification-policy-v3.json'
    time.focus = [
        'empty', 'one-stored', 'seven-stored-prefix', '320-overlap', 'random-64', 'random-1024',
        'prefix-random-1024', 'prefix-periodic-1024', 'prefix-periodic-4096', 'prefix-natural-4096',
        'prefix-natural-8192', 'prefix-natural-16384', 'prefix-natural-32769', 'prefix-random-4096',
        'heldout-mixed-131073',
    ]
    return time


def summarize(folder, kind):
    replay = read(folder / f'{kind}-replay.json')
    near = ('320-overlap', 'prefix-natural-4096', 'prefix-natural-8192',
            'prefix-periodic-1024', 'prefix-random-1024')
    rows = []
    for run in replay['runs']:
        source = {row['case']: row for row in read(Q / run['id'] / 'summary.json')}
        ratios = {name: source[name]['paired_x_libdeflate'] for name in near}
        wins = {k: row['point_estimate_wins'] for k, row in run['peers'].items()}
        losses = {k: row['lost_cases'] for k, row in run['peers'].items()}
        rows.append({'order': run['order'], 'focus_ratios_vs_libdeflate': ratios,
                     'wins': wins, 'losses': losses, 'receipts': run['id']})
    summary = {'kind': kind, 'policy_version': 3, 'decision': replay['decision'],
               'preserves_peer_wins': not any(v for row in rows for v in row['losses'].values()),
               'both_targets_win': all(row['focus_ratios_vs_libdeflate'][name]['mech-experiment'] < 1
                                       for name in ('320-overlap', 'prefix-natural-8192') for row in rows),
               'runs': rows, 'rejected': replay['rejected'], 'unresolved': replay['unresolved']}
    write(folder / 'screen.json', summary)
    for row in rows:
        print(row['order'], row['wins'], [(k, round(v['mech-experiment'], 6))
              for k, v in row['focus_ratios_vs_libdeflate'].items()], 'lost', row['losses'], flush=True)
    print(summary['decision'], flush=True)


def measure(role, kind):
    time = runtime_modules()
    inputs = HERE / (role + '-layout')
    worker = inputs / 'bound'
    bound = read(worker / 'runtime.json')
    assert sha(worker / 'mech-worker') == bound['worker_sha256']
    assert sha(worker / 'program.o') == bound['object_sha256']
    checks = HERE / 'linked/linked-proofs/result.json'
    assert read(checks)['matches_success_observations'] == 162
    if kind == 'full':
        screened = read(inputs / 'focus/screen.json')
        assert screened['preserves_peer_wins'] and screened['both_targets_win']
        assert read(checks)['peak_stack_bytes'][role] <= 5280
    time.control = QUALIFIED if kind == 'full' else HERE / CONTROL / 'bound'
    folder = inputs / kind
    label = f'short-visit-{role}-{kind}'
    time.prepare(label, worker, folder)
    plan = read(folder / 'plan.json')
    for key in ('source_commit', 'review_commit', 'source_reproduction', 'metadata_reconciliation',
                'original_focus', 'original_repeatability', 'history'):
        del plan[key]
    contract = folder / 'contract.json'
    write(contract, {'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'complete compression through private heap cleanup',
        'fixture_inventory_sha256': read(time.policy_file)['fixture_inventory_sha256']})
    plan['identities']['contract_sha256'] = sha(contract)
    plan.update(source_pin=BASE, policy_version=3,
        objective='Measure shorter vector entry or paired scalar continuation for the exact compressor.',
        driver=time.ref(Path(__file__).resolve()), controller=time.ref(Path(time.__file__)),
        linked_contract=time.ref(checks), max_peak_stack_bytes=5280, load_average_before=os.getloadavg(),
        runs=[{'id': f'{label}-{order}', 'kind': kind, 'order': order, 'blocks_per_case': 96}
              for order in ('forward', 'reverse')])
    write(folder / 'plan.json', plan)
    (folder / 'plan.sha256').write_text(sha(folder / 'plan.json') + '\n')
    with (folder / 'acquire.log').open('w') as log, redirect_stdout(log):
        time.acquire(plan, worker, kind)
    write(folder / 'post-acquisition.json', {'load_average': os.getloadavg()})
    with (folder / 'replay.log').open('w') as log, redirect_stdout(log):
        time.replay(plan, folder, kind)
    summarize(folder, kind)


def main():
    action, *roles = sys.argv[1:]
    assert roles and set(roles) <= MODE.keys()
    if action == 'emit':
        emit_roles(roles)
    elif action == 'check':
        check_roles(roles)
    elif action == 'count-placed':
        for role in roles:
            study.measure_counts(role + '-layout')
    else:
        assert action in ('focus', 'full')
        for role in roles:
            measure(role, action)


if __name__ == '__main__':
    main()
