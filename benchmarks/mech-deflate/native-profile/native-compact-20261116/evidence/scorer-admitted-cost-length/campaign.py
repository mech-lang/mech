"""Compare the smaller placed scorer with and without a split cost check."""

from contextlib import redirect_stdout
from importlib import import_module
import os
from pathlib import Path
import runpy
import sys

from stage import BASE, HERE, PROBE, Q, ROADMAP, read, sha, write

compiler = runpy.run_path(str(HERE / 'build.py'), run_name='scorer_cost_length_compiler')
COMPILER, OUT, PLAN, REFERENCE = (compiler[name] for name in ('COMPILER', 'OUT', 'PLAN', 'REFERENCE'))
OLD = ROADMAP / 'scorer-vector-latch'
OLD_BINARY = OLD / 'latch-kept-root-layout'
CONTROL = 'cost-latch-control'
RAW = 'cost-split-unplaced'
CANDIDATE = 'cost-split-placed'

sys.path.insert(0, str(ROADMAP / 'ordered-scorer'))
study = import_module('study')
driver = study.driver
driver.HERE = HERE
driver.PIN = BASE
driver.PLAN = HERE / 'control-plan.json'
study.HERE = HERE
for module in (study.rebase, study.scheduler, study.count, study.fit):
    module.HERE = HERE

BASELINE_MODE = driver.HORIZONTAL | driver.REVERSE | {'MECH_ORDERED_VECTOR_LATCH_TEST': '1'}


def pin_inputs():
    build = read(OUT / 'result.json')
    assert build['base'] == BASE and build['compiler_sha256'] == sha(COMPILER)
    assert build['comparison_plan_sha256'] == sha(PLAN)
    baseline = read(PLAN)['settings'][REFERENCE]
    assert baseline['artifact_sha256'] == build['artifacts_disabled']
    bound = {'actors': {'mech-verified-flowgraph': baseline}}
    if driver.PLAN.exists():
        assert read(driver.PLAN) == bound
    else:
        write(driver.PLAN, bound)


def emit_layout():
    pin_inputs()
    driver.emit(CONTROL, BASELINE_MODE, COMPILER, OLD / 'latch-test-scorer-profile.json')
    old = read(OLD_BINARY / 'receipt.json')
    control = read(HERE / CONTROL / 'receipt.json')
    for key in ('worker_sha256', 'metadata_sha256', 'bytecode_sha256'):
        assert old[key] == control[key], (key, old[key], control[key])
    assert old['layout']['object_sha256'] == control['layout']['object_sha256']
    arm = BASELINE_MODE | {PROBE: '1'}
    inspect = {**arm, 'MECH_INSPECT_PHASE_CLIF': str(HERE / (driver.SCORER[1:] + '.clif'))}
    driver.emit(RAW, inspect, COMPILER)
    needle = f'{driver.SCORER[1:]} split admitted cost checks: 2'
    assert needle in (HERE / RAW / 'compile.log').read_text()

    study.measure_counts(RAW)
    placed = study.profiles(RAW)
    driver.emit(CANDIDATE, arm, COMPILER, placed)
    bind_placed()


def bind_placed():
    control = read(HERE / CONTROL / 'receipt.json')
    candidate = read(HERE / CANDIDATE / 'receipt.json')
    placed = HERE / (RAW + '-scorer-profile.json')
    assert candidate['bytecode_sha256'] == control['bytecode_sha256']
    fixed = '_mech_compact_closed_checked'
    keys = ('sha256', 'extent', 'relocations')
    assert all(candidate['layout']['symbols'][fixed][key] == control['layout']['symbols'][fixed][key]
               for key in keys)
    changes = sorted(set(control['layout']['symbols']) | set(candidate['layout']['symbols']))
    affected = [name for name in changes
                if any(control['layout']['symbols'].get(name, {}).get(key) !=
                       candidate['layout']['symbols'].get(name, {}).get(key) for key in keys)]
    assert affected == [driver.SCORER], affected
    write(HERE / 'build-comparison.json', {'base': BASE, 'control': control,
        'candidate': candidate, 'inherited_layout': {'path': str(OLD / 'latch-test-scorer-profile.json'),
                                                   'sha256': sha(OLD / 'latch-test-scorer-profile.json')},
        'candidate_layout': {'path': str(placed), 'sha256': sha(placed)}, 'changed_symbols': affected,
        'checked_root': control['layout']['symbols'][fixed],
        'clif': str(HERE / (driver.SCORER[1:] + '.clif'))})
    check_workers()


def check_workers():
    checked = HERE / 'checks'
    roles = {'maintained': driver.TEMPLATE, CONTROL: HERE / CONTROL / 'bound',
             CANDIDATE: HERE / CANDIDATE / 'bound'}
    for role, source in roles.items():
        output = checked / 'worker-checks' / role
        binding = driver.link.link_worker(source, source, output)
        assert binding['worker_sha256'] == sha(source / 'mech-worker')
        driver.link.check_worker(output, f'admitted-cost-{role}')
    assert driver.link.check_contracts(checked, tuple(roles)) == 0
    result = read(checked / 'linked-proofs/result.json')
    assert result['matches_success_observations'] == 162 and len(result['checked_failure_cases']) == 6
    print(result['peak_stack_bytes'], flush=True)


def timed(kind):
    assert kind in ('focus', 'full')
    linked = HERE / 'checks/linked-proofs/result.json'
    checks = read(linked)
    assert checks['matches_success_observations'] == 162
    sys.path.insert(0, str(Q / 'source-frame-copy-tails'))
    time = import_module('full-timing')
    time.policy_file = time.packet / 'qualification-policy-v3.json'
    focused = runpy.run_path(str(ROADMAP / 'scorer-short-visits/evaluate.py'), run_name='focused_comparison')
    # Borrow the frozen case membership and result summary without adjusting the
    # plan or safety rules from the previous runtime comparison.
    time.focus = focused['runtime_modules']().focus
    time.control = driver.TEMPLATE if kind == 'full' else HERE / CONTROL / 'bound'
    arm = HERE / CANDIDATE / 'bound'
    assert sha(arm / 'mech-worker') == read(arm / 'runtime.json')['worker_sha256']
    folder = HERE / kind
    prefix = f'admitted-cost-{kind}'
    if kind == 'full':
        focused_result = read(HERE / 'focus/screen.json')
        assert focused_result['preserves_peer_wins'] and focused_result['both_targets_win']
        assert checks['peak_stack_bytes'][CANDIDATE] <= 5280
        assert read(HERE / 'boundary/result.json')['cases'] > 0
    time.prepare(prefix, arm, folder)
    plan = read(folder / 'plan.json')
    for key in ('source_commit', 'review_commit', 'source_reproduction', 'metadata_reconciliation',
                'original_focus', 'original_repeatability', 'history'):
        del plan[key]
    contract = folder / 'contract.json'
    write(contract, {'scope': 'diagnostic', 'level': 12,
        'timing_boundary': 'complete compression through private heap cleanup',
        'fixture_inventory_sha256': read(time.policy_file)['fixture_inventory_sha256']})
    plan['identities']['contract_sha256'] = sha(contract)
    plan.update(source_pin=BASE, policy_version=3, objective='Price the split of an admitted cost extent check.',
        driver=time.ref(Path(__file__).resolve()), controller=time.ref(Path(time.__file__)),
        compiler_build=time.ref(OUT / 'result.json'), linked_contract=time.ref(linked),
        max_peak_stack_bytes=5280, load_average_before=os.getloadavg(),
        runs=[{'id': f'{prefix}-{order}', 'kind': kind, 'order': order, 'blocks_per_case': 96}
              for order in ('forward', 'reverse')])
    write(folder / 'plan.json', plan)
    (folder / 'plan.sha256').write_text(sha(folder / 'plan.json') + '\n')
    with (folder / 'acquire.log').open('w') as log, redirect_stdout(log):
        time.acquire(plan, arm, kind)
    write(folder / 'post-acquisition.json', {'load_average': os.getloadavg()})
    with (folder / 'replay.log').open('w') as log, redirect_stdout(log):
        time.replay(plan, folder, kind)
    focused['summarize'](folder, kind)


if __name__ == '__main__':
    command, = sys.argv[1:]
    if command == 'emit':
        emit_layout()
    elif command == 'bind':
        bind_placed()
    else:
        timed(command)
