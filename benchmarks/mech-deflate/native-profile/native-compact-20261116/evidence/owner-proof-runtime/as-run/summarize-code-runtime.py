"""Verify paired timing packets against the emitted object and linked worker."""
from importlib import import_module
from pathlib import Path
from statistics import geometric_mean
import argparse
import json

from action_origin_witness import check_witness


q = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('label')
parser.add_argument('--reference', default='stable-reuse-selected-header')
parser.add_argument('--focus-suffix', default='focus')
args = parser.parse_args()
runner = import_module('checked-controller-results')
macho = import_module('reachability-census')


def read(path):
    return json.loads(path.read_text())


def artifact(label):
    folder = q / f'{label}-native' / 'final-candidate'
    receipt = read(folder / 'receipt.json')
    checks = {
        'program.o': 'object_sha256',
        'metadata.json': 'metadata_sha256',
        'mech-worker': 'worker_sha256',
    }
    assert all(runner.sha(folder / file) == receipt[field] for file, field in checks.items())
    _, sections, _ = macho.load(folder / 'program.o')
    text_size = sections[macho.TEXT][2]
    assert receipt['object_text_bytes'] == text_size
    assert receipt['object_text_x_roc'] == text_size / receipt['roc_object_text_bytes']
    proof = read(Path(receipt['proof']))
    assert proof['verified'] and proof['complete_smokes'] == receipt['verified_calls'] == 162
    assert proof['worker_sha256'] == receipt['worker_sha256']
    assert proof['metadata_sha256'] == receipt['metadata_sha256']
    stack_path = q / f'{label}-stack.json'
    stack = read(stack_path)[str(folder.relative_to(q))]
    assert stack.keys() == runner.oracle.keys()
    assert all(set(calls) == {'0', '1'} for calls in stack.values())
    failures_path = q / f'{label}-checked-failures.json'
    failures = read(failures_path)
    assert len(failures['cases']) == 6
    assert failures['actors'][-1] == str(folder.relative_to(q))
    for evidence in failures['cases'].values():
        old, new = evidence['per_actor_outcomes']
        assert old['status'] == new['status'] != 0
        origins = evidence['matched_source_origins']
        if witness := evidence.get('checked_action_witness'):
            assert check_witness(
                {'checked_action_witnesses': {str(old['origin']): witness}},
                q / failures['actors'][0], q / failures['actors'][-1],
                old['origin'], new['origin'],
            )
        else:
            assert all(origins[0][key] == origins[1][key] for key in ('node', 'local'))
    return dict(receipt=receipt, code_text_x_roc=receipt['object_text_x_roc'],
                read_only_data_bytes=sections.get((b'__TEXT', b'__const'), (0, 0, 0))[2],
                peak_stack_bytes=max(value for calls in stack.values() for value in calls.values()),
                correctness=receipt['proof'], stack=str(stack_path), failures=str(failures_path))


control, candidate = artifact(args.reference), artifact(args.label)
assert control['receipt']['roc_object_text_bytes'] == candidate['receipt']['roc_object_text_bytes']


def run(suffix):
    result = runner.compare(f'{args.label}-{suffix}', allow_source_counter_changes=True)
    for actor, actual in (('mech-control', control), ('mech-experiment', candidate)):
        pinned = result['pins']['actors'][actor]
        assert pinned['binary_sha256'] == actual['receipt']['worker_sha256']
        assert pinned['metadata_sha256'] == actual['receipt']['metadata_sha256']
    return result


full = [run('full'), run('full-reverse')]
focused = [run(args.focus_suffix), run(f'{args.focus_suffix}-reverse')]
assert all(len(result['per_case']) == 81 for result in full)
assert full[0]['per_case'].keys() == full[1]['per_case'].keys()
timings = {case: [run['per_case'][case]['candidate_over_control'] for run in full]
           for case in full[0]['per_case']}
regressions = {case: values for case, values in timings.items() if min(values) > 1.005}
size = candidate['code_text_x_roc']
report = dict(
    machine='Apple M1', roc_baseline=1.0,
    reference=control, candidate=candidate,
    code_reduction=1 - size/control['code_text_x_roc'], reduction_to_roc_parity=1 - 1/size,
    full_runs=full, focused_runs=focused,
    timed_regressions_over_half_percent_in_both_orders=regressions,
)
path = q / f'{args.label}-native' / 'full-timing.json'
path.write_text(json.dumps(report, indent=2) + '\n')
print(f"{control['code_text_x_roc']:.4f}× Roc to {size:.4f}× Roc")
print('Largest paired relative timing', max((geometric_mean(v), k) for k, v in timings.items()))
print('Sustained regressions over 0.5%', regressions)
