from pathlib import Path
import datetime
import gzip
import hashlib
import json
import math
import statistics
import subprocess
import time

root = Path(__file__).resolve().parent
workspace = Path('/private/tmp/mech-deflate-linear-stack')
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
active = subprocess.check_output(['ps', '-Ao', 'comm='], text=True).splitlines()
assert not [p for p in active if Path(p.strip()).name in ['component-v2-tests', 'selected-native-tests', 'selected-bits-tests', 'rustc', 'cargo', 'complete-reality', 'compression-application', 'word-updates', 'round01-pinned-control', 'artifact_review', 'native_frame_transport']]
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=workspace, text=True).strip()
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=workspace, text=True).strip()
artifact = Path('/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc')
assert digest(artifact) == 'a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7'
assert digest(artifact.with_suffix('.mec')) == '331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9'
pins = json.loads((workspace / 'benchmarks/mech-deflate/results/2026-10-28-compressor-comparison/build.json').read_text())
package = Path('/private/tmp/roc-deflate-repro-20261024/roc-deflate')
assert all(digest(package / name) == expected for name, expected in pins['reference_package_sha256'].items())
commands = []

def execute(label, command, timeout=240):
    assert not (root / (label + '.stdout')).exists(), 'never overwrite observations'
    started = time.monotonic()
    with (root / (label + '.stdout')).open('wb') as out, (root / (label + '.stderr')).open('wb') as err:
        result = subprocess.run(command, stdout=out, stderr=err, timeout=timeout)
    commands.append({'label': label, 'command': command, 'exit': result.returncode, 'process_seconds': time.monotonic() - started})
    (root / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n')
    result.check_returncode()
    return (root / (label + '.stdout')).read_text()

reference = Path('/private/tmp/mech-deflate-profiling-20261028/roc-profile')
assert digest(reference) == '3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324'
raw = execute('roc', [str(reference), '12'])
ref_rows = [line.split('\t') for line in raw.splitlines() if line.startswith('rep\troc-deflate\t')]
assert len(ref_rows) == 20000
ref = statistics.median(int(row[3]) for row in ref_rows) / 1e9
assert ref > 0
print('ROC_SECONDS', ref, flush=True)

variant = root / 'independent-cost-tables.mbc'
assert digest(variant) == '33168977b1d9a0c873a57d6c8e6bd2f87035847cd4619323277403549a14a358'
assert digest(variant.with_suffix('.mec')) == '278c8bf6ec66d6406fb922fbce936f89a3917fa2b2c9f8099da1fa07ceda1e10'
binary = Path('/private/tmp/mech-mixed-scalar-state-round08-20261028/current')
expected = '0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570'
controls = [('before', binary, expected, artifact), ('after', binary, expected, variant)]
results = {'comparison_scope': 'SOURCE EXPERIMENT: identical compiler executable, selected catalog and allocator; only independent cost-table expressions differ', 'head': head, 'baseline': 'Roc = 1x', 'previous_round_multiple': 454.8915228070175, 'input_bytes': 8, 'level': 12, 'reference_seconds': ref, 'reference_samples': len(ref_rows), 'reference_sha256': digest(reference), 'artifact_sha256': digest(artifact), 'source_sha256': digest(artifact.with_suffix('.mec')), 'timing_boundary': 'unchanged evaluate_into-only harness; output reserve13/clear/rebind/JIT/verification/drop excluded', 'allocator': 'same pinned mimalloc2.1.7, no allocation probe', 'runtime_policy': 'existing diagnostic NoLimits unchanged', 'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'controls': {}}
for label, binary, expected, subject_artifact in controls:
    assert digest(binary) == expected
    observations = [json.loads(line) for line in execute(label, [str(binary), str(subject_artifact), '3']).splitlines()]
    primary = [o for o in observations if not o['warmup'] and not o['profile']]
    assert len(primary) == 3 and len(observations) == 5
    assert all(o['exact_reference'] and o['independent_inflation'] and o['capacity'] == 13 for o in observations)
    assert (root / (label + '.stderr')).read_text().endswith('CLEANUP bytes=0 nodes=0\n')
    assert len({o['work'] for o in observations}) == 1
    if label == 'before':
        assert observations[0]['work'] == 875610515844
    median = statistics.median(o['seconds'] for o in primary)
    results['controls'][label] = {'binary': str(binary), 'sha256': expected, 'artifact_sha256': digest(subject_artifact), 'source_sha256': digest(subject_artifact.with_suffix('.mec')), 'median_seconds': median, 'multiple_of_roc': median / ref, 'primary': primary, 'all_observations': observations, 'cleanup': True}
    (root / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    print(label, json.dumps({'median_seconds': median, 'multiple_of_roc': median / ref}), flush=True)

# All observations in this separate process are diagnostic and excluded.
repetitions = max(4, min(10000, math.ceil(4 / results['controls']['after']['median_seconds'])))
command = [str(binary), str(variant), str(repetitions)]
started = time.monotonic()
with (root / 'profile.stdout').open('wb') as out, (root / 'profile.stderr').open('wb') as err:
    process = subprocess.Popen(command, stdout=out, stderr=err)
    sampled = False
    while process.poll() is None:
        if not sampled and 'MEASURING 1\n' in (root / 'profile.stderr').read_text():
            sampled = True
            sample_command = ['sample', str(process.pid), '2', '-file', str(root / 'after.sample.txt')]
            sampled_result = subprocess.run(sample_command, capture_output=True, text=True)
            (root / 'profile-command.json').write_text(json.dumps({'command': command, 'repetitions': repetitions, 'sample_command': sample_command, 'sample_exit': sampled_result.returncode, 'output': sampled_result.stdout + sampled_result.stderr, 'all_observations_excluded_from_primary': True}, indent=2) + '\n')
        if time.monotonic() - started > 240:
            process.terminate()
            raise RuntimeError('bounded small-case profiling process exceeded 240 seconds')
        time.sleep(0.005)
    assert process.wait() == 0
assert sampled and (root / 'after.sample.txt').stat().st_size > 1000
assert (root / 'profile.stderr').read_text().endswith('CLEANUP bytes=0 nodes=0\n')
results['logical_work_delta'] = results['controls']['after']['all_observations'][0]['work'] - results['controls']['before']['all_observations'][0]['work']
results['logical_work_note'] = 'This source optimization changes operations and allocations. Preserve both logical debit totals; do not attribute this to the compiler.'
results['after_over_before'] = results['controls']['after']['median_seconds'] / results['controls']['before']['median_seconds']
results['speedup'] = 1 / results['after_over_before']
results['profile_repetitions'] = repetitions
results['profile_primary_observations'] = 0
(root / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print('COMPLETE', json.dumps({'before_multiple_of_roc': results['controls']['before']['multiple_of_roc'], 'after_multiple_of_roc': results['controls']['after']['multiple_of_roc'], 'after_over_before': results['after_over_before'], 'profile_repetitions': repetitions}), flush=True)
