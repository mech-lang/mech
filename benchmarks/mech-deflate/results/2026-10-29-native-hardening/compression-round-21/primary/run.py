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
assert not [p for p in active if Path(p.strip()).name in ['unit-tests3', 'component-v2-tests', 'selected-native-tests', 'selected-bits-tests', 'rustc', 'cargo', 'complete-reality', 'compression-application', 'word-updates', 'round01-pinned-control', 'artifact_review', 'native_frame_transport']]
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=workspace, text=True).strip()
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=workspace, text=True).strip()
artifact = Path('/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc')
assert digest(artifact) == 'a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7'
assert digest(artifact.with_suffix('.mec')) == '331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9'
pins = json.loads((workspace / 'benchmarks/mech-deflate/results/2026-10-28-compressor-comparison/build.json').read_text())
package = root.parent / 'reference-pinned'
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

controls = [('before', root / 'before', '1e36ae06aee5eb4be8af26070868c32683040564ca89d7cb877d2a1a56a4b03b'),
            ('after', root / 'current', json.loads((root / 'current-build.json').read_text())['binary_sha256'])]
results = {'comparison_scope': 'same default selected-operation profile and pinned compressor; multiple current array owners in private numeric tuple results; complete per-field provenance/version proof; unchanged pinned compressor', 'head': head, 'baseline': 'Roc = 1x', 'previous_round_multiple': 102.7075332167832, 'input_bytes': 8, 'level': 12, 'reference_seconds': ref, 'reference_samples': len(ref_rows), 'reference_sha256': digest(reference), 'artifact_sha256': digest(artifact), 'source_sha256': digest(artifact.with_suffix('.mec')), 'timing_boundary': 'unchanged evaluate_into-only harness; output reserve13/clear/rebind/JIT/verification/drop excluded', 'allocator': 'same pinned mimalloc2.1.7, no allocation probe', 'runtime_policy': 'existing diagnostic NoLimits unchanged', 'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'controls': {}}
results['design'] = 'Fixed four balanced alternating-order process pairs; three primary observations per control per process; all observations retained.'
for label,binary,expected in controls:
    assert digest(binary)==expected
    results['controls'][label]={'binary':str(binary),'sha256':expected,'primary':[],'all_observations':[],'pairs':[],'cleanup':True}
results['pairs']=[]
for pair in range(4):
    order=controls if pair%2==0 else list(reversed(controls))
    pair_result={'pair':pair,'order':[x[0] for x in order]}
    for label,binary,expected in order:
        observations=[json.loads(line) for line in execute(f'pair{pair}-{label}',[str(binary),str(artifact),'3']).splitlines()]
        primary=[o for o in observations if not o['warmup'] and not o['profile']]
        assert len(primary)==3 and len(observations)==5
        assert all(o['exact_reference'] and o['independent_inflation'] and o['capacity']==13 for o in observations)
        assert (root/f'pair{pair}-{label}.stderr').read_text().endswith('CLEANUP bytes=0 nodes=0\n')
        assert len({o['work'] for o in observations})==1
        if label=='before':assert observations[0]['work']==306669833472
        control=results['controls'][label];control['primary']+=primary;control['all_observations']+=observations
        median=statistics.median(o['seconds'] for o in primary)
        control['pairs'].append({'pair':pair,'median_seconds':median,'observations':observations})
        pair_result[label]=median
    pair_result['after_over_before']=pair_result['after']/pair_result['before']
    results['pairs'].append(pair_result)
    (root/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    print('PAIR',json.dumps(pair_result),flush=True)
for label in ['before','after']:
    control=results['controls'][label];assert len(control['primary'])==12
    control['median_seconds']=statistics.median(o['seconds'] for o in control['primary'])
    control['multiple_of_roc']=control['median_seconds']/ref
    assert len({o['work'] for o in control['all_observations']})==1
results['median_paired_ratio']=statistics.median(p['after_over_before'] for p in results['pairs'])
(root/'results.json').write_text(json.dumps(results,indent=2)+'\n')

# All observations in this separate process are diagnostic and excluded.
repetitions = max(4, min(10000, math.ceil(4 / results['controls']['after']['median_seconds'])))
command = [str(root / 'current'), str(artifact), str(repetitions)]
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
assert results['controls']['after']['all_observations'][0]['work'] > 0
results['logical_work_note'] = 'IR124 extends exact private flat tuple numeric results to every current array field. Existing all-owner freeze and terminal synchronization are reused. Existing work93 prices, ABI35 and bytecode14 remain; current owners, capacities and executed physical routes remain authoritative.'
results['after_over_before'] = results['controls']['after']['median_seconds'] / results['controls']['before']['median_seconds']
results['speedup'] = 1 / results['after_over_before']
results['profile_repetitions'] = repetitions
results['profile_primary_observations'] = 0
(root / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print('COMPLETE', json.dumps({'before_multiple_of_roc': results['controls']['before']['multiple_of_roc'], 'after_multiple_of_roc': results['controls']['after']['multiple_of_roc'], 'after_over_before': results['after_over_before'], 'profile_repetitions': repetitions}), flush=True)
