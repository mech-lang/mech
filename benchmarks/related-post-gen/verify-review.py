#!/usr/bin/env python3
"""Verify evidence, its historical compiler, and any current integration inventory."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

packet = Path(__file__).resolve().parent
root = packet.parents[1]
manifest = json.loads((packet / 'review/manifest.json').read_text())

def verify(base, name, expected):
    path = (base / name).resolve()
    if not path.is_relative_to(base.resolve()):
        raise SystemExit(f'Invalid manifest path: {name}')
    if not path.is_file():
        raise SystemExit(f'Missing published file: {name}')
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f'Hash mismatch: {name}')

for name, item in manifest['files'].items():
    verify(packet, name, item['sha256'])
    if item.get('compression') == 'gzip':
        assert hashlib.sha256(gzip.decompress((packet/name).read_bytes())).hexdigest() == item['uncompressed_sha256'], name
# The current branch includes newer compiler work. Historical timings belong
# to the recorded Git tree, not to whatever compiler happens to be checked out.
revision = manifest['measured_compiler']
assert len(revision) == 40 and all(c in '0123456789abcdef' for c in revision)
names = list(manifest['compiler_files'])
for name in names:
    assert (root/name).resolve().is_relative_to(root.resolve()), name
objects = subprocess.run(
    ['git', 'cat-file', '--batch'], cwd=root,
    input=''.join(f'{revision}:{name}\n' for name in names).encode(),
    capture_output=True, check=True,
).stdout
at = 0
for name in names:
    end = objects.index(b'\n', at)
    header = objects[at:end].split()
    if len(header) != 3 or header[1] != b'blob':
        raise SystemExit(f'Historical compiler blob unavailable: {revision}:{name}')
    size = int(header[2])
    at = end + 1
    data = objects[at:at+size]
    assert len(data) == size and objects[at+size:at+size+1] == b'\n', name
    assert hashlib.sha256(data).hexdigest() == manifest['compiler_files'][name], name
    at += size + 1
assert at == len(objects)

integration = packet / 'integration/source-inventory.json'
current_files = 0
if integration.exists():
    current = json.loads(integration.read_text())
    for name, expected in current['files'].items():
        verify(root, name, expected)
    current_files = len(current['files'])

# Validate the corrected audit from raw reports, not a printed success label.
evidence = packet / 'integration/evidence'
audit_path = evidence / 'funding-audit-audit-summary.json'
if audit_path.exists():
    audit = json.loads(audit_path.read_text())
    assert audit['status'] == 'passed'
    def control_rows(text):
        return [{k: v for k, v in json.loads(line).items()
                 if k not in ('seconds', 'seconds_per_evaluation')}
                for line in text.splitlines() if line.startswith('{')]
    current = control_rows(json.loads((evidence / 'current-control-one-kib-run.json').read_text())['stdout'])
    for name in ('ir226-control-one-kib-run.json', 'funding-disabled-one-kib-run.json'):
        assert control_rows(json.loads((evidence / name).read_text())['stdout']) == current, name
    assert control_rows((evidence / 'funding-audit-ir226-repeat.stdout.log').read_text()) == current
    assert len(current) == 5
    old_comparison = json.loads((evidence / 'ir226-control-one-kib-comparison.json').read_text())
    assert old_comparison['all_nontiming_fields_equal_IR225'] is False
    assert len(old_comparison['differences']) == 3
    assert json.loads((evidence / 'funding-audit-audit.json').read_text())['exit'] == 0

screen_path = evidence / 'current-screen-report.json'
if screen_path.exists():
    screen = json.loads(screen_path.read_text())
    for phase, count in (('initial', 405), ('confirmation', 675),
                         ('extended', 1395), ('resources', 357)):
        report = json.loads(gzip.decompress((evidence / f'current-{phase}.json.gz').read_bytes()))
        assert len(report['samples']) == count
        assert all(row['exit'] == 0 and row['exact_rust_output']
                   for row in report['samples'])
        if phase == 'extended':
            assert report['summary'] == screen['extended_31_rounds']
            assert report['matched_native_reference'] == screen['references']['matched']
            assert report['whole_process_reference'] == screen['references']['whole']
    assert screen['qualification']['cohort_outputs_exact'] == 2832
    assert screen['qualification']['genuine_old_objects_rejected'] == 25
    assert screen['paired']['extended']['mech-before-4']['wall_seconds']['wins'] == 19
    assert screen['selected'] is False and screen['goal_complete'] is False

latest_path = evidence / 'ir228-screen-report.json'
if latest_path.exists():
    latest = json.loads(latest_path.read_text())
    assert latest['identity'] == 'IR228/work115/ABI41/image9'
    for phase, count in (('initial', 405), ('confirmation', 675), ('resources', 357)):
        report = json.loads(gzip.decompress((evidence / f'ir228-{phase}.json.gz').read_bytes()))
        assert len(report['samples']) == count
        assert all(row['exit'] == 0 and row['exact_rust_output']
                   for row in report['samples'])
        if phase == 'confirmation':
            assert report['summary'] == latest['confirmation_15_rounds']
    assert latest['qualification']['genuine_old_objects_rejected'] == 26
    assert latest['qualification']['cohort_outputs_exact'] == 1437
    assert all(row['complete_native_text_byte_identical']
               for row in latest['native_text_equivalence'].values())
    for kind in ('scalar', 'one-kib'):
        result = json.loads((evidence / f'ir228-control-{kind}-comparison.json').read_text())
        assert result['all_nontiming_fields_equal_IR227']
    assert latest['selected'] is False and latest['goal_complete'] is False

current_status = packet / 'integration/ir231-review-status.json'
if current_status.exists():
    status = json.loads(current_status.read_text())
    assert status['identity'] == 'IR231/work115/ABI41/image9'
    assert status['target_pr'] == 973
    assert status['gpu_passed'] == 841
    assert len(status['inherited_gpu_failures']) == 2
    assert status['current_performance_cohort_run'] is True
    assert status['new_performance_gain_claim'] is False
    assert status['selected'] is False and status['goal_complete'] is False
    before = json.loads((evidence / 'ir231-unfixed-confirmation.json').read_text())
    assert before['inherited_miscompile_reproduced']
    assert json.loads((evidence / 'ir231-unfixed-reproducer.json').read_text())['exit'] == 101
    after = json.loads((evidence / 'ir231-gpu-focus-ordered_reduction.json').read_text())
    assert after['exit'] == 0
    full = json.loads((evidence / 'ir231-gpu-test-run.json').read_text())
    assert full['expected_failures_exact'] and not full['new_failures']
    for kind, count in [('scalar', 128), ('one-kib', 5)]:
        def rows(path):
            return [{k: v for k, v in json.loads(line).items()
                     if k not in ('seconds', 'seconds_per_evaluation')}
                    for line in json.loads(path.read_text())['stdout'].splitlines()
                    if line.startswith('{')]
        old = rows(evidence / f'ir228-control-{kind}-run.json')
        new = rows(evidence / f'ir231-control-{kind}-run.json')
        assert len(old) == len(new) == count and old == new
    native = json.loads((evidence / 'ir231-native-text-equivalence.json').read_text())
    assert len(native) == 4
    assert all(x['complete_native_text_byte_identical'] for x in native.values())

experiments = evidence / 'boolean-experiments'
if experiments.exists():
    experiment = json.loads((experiments / 'summary.json').read_text())
    assert experiment['selected'] is False and experiment['goal_complete'] is False
    assert experiment['qualification']['IR232_gpu_passed'] == 843
    assert experiment['qualification']['IR233_full_gpu_and_compression_controls_not_run']
    total = 0
    for label, expected in [('IR232-latency', 248), ('IR232-resources', 63),
                            ('IR233-latency', 279), ('IR233-resources', 70)]:
        run = json.loads(gzip.decompress((experiments / (label+'.json.gz')).read_bytes()))
        assert len(run['samples']) == expected
        assert all(row['exit'] == 0 and row['exact_rust_output'] for row in run['samples'])
        assert run['execution_order']['mode'] == 'randomized'
        assert len(run['execution_order']['rounds']) == (31 if 'latency' in label else 7)
        total += expected
    assert total == 660
    latest = json.loads(gzip.decompress((experiments / 'IR233-latency.json.gz').read_bytes()))
    assert latest['matched_native_reference']['variant'] == 'rust-mech-control-u8-ids-u32'
    assert latest['whole_process_reference']['variant'] == 'rust-mech-control-u32-casts'
    assert latest['summary']['mech-parent-4']['median_post_json_seconds'] == .002961916
    assert latest['summary']['mech-width-4']['median_post_json_seconds'] == .002954208
    assert experiment['paired']['mech-after-4']['mech-parent-4']['post_json_seconds']['wins'] == 14
    assert experiment['paired']['mech-width-4']['mech-parent-4']['post_json_seconds']['wins'] == 17

report = json.loads((packet / 'results/comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json').read_text())
assert report['summary']['mech-after-4']['median_post_json_seconds'] == 0.003140833
assert report['matched_native_reference']['median_seconds'] == 0.001651875
assert report['whole_process_reference']['variant'] == 'rust-mech-control-streaming'
print(json.dumps(dict(status='verified', evidence_files=len(manifest['files']),
                      historical_compiler_files=len(manifest['compiler_files']),
                      historical_source='recorded_git_tree',
                      current_integration_source_files=current_files,
                      measured_compiler=manifest['measured_compiler'],
                      executes_benchmark=False)))
