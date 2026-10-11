"""Build shorter pure Mech sources and compare their complete state to baselines."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

HERE = Path(__file__).resolve().parent
NBODY = HERE.parents[1]
ROOT = NBODY.parents[2]
COMPARISON = ROOT / 'benchmarks/clbg/array-comparison/nbody-20261009'
DRIVER = NBODY / 'mech-driver/target/release/mech-clbg-nbody-compiled'
CLANG = Path('/opt/homebrew/opt/llvm@22/bin/clang-22')
SDK = '/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk'
FROZEN = ROOT / 'benchmarks/clbg/function-application-audit/20261009T171437Z/application-v3/builds/nbody'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

report = dict(created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
              driver_sha256=sha(DRIVER), compiler_files={}, programs={}, checks=[])
for path in ('hosts/gpu/src/fused_f64/mod.rs', 'hosts/gpu/src/fused_f64/runner.rs'):
    report['compiler_files'][path] = sha(ROOT / path)

specs = [
    ('compact', 'nbody-matrix-short.mec', 20, None, None),
    ('optimized', 'nbody-matrix-pairwise.mec', 25, '--pack-state-tails', None),
    ('compact-baseline', 'nbody.mec', 32, None, 'compact-baseline-before.o'),
    ('optimized-baseline', 'nbody-pairwise-mass.mec', 37, '--pack-state-tails', 'pairwise-mass-tails-before.o'),
]
for key, name, expected_loc, extra_flag, frozen_name in specs:
    source = NBODY / name
    code = source.read_text()
    lines = [s for s in code.splitlines() if s.strip() and not s.lstrip().startswith('--')]
    assert len(lines) == expected_loc and all(s.count(':=') <= 1 for s in lines)
    if not key.endswith('baseline'):
        assert lines[-1] == 'energy' and '<+' not in code
    c = HERE / (key + '.c')
    obj = HERE / (key + '.o')
    binary = HERE / key
    command = [str(DRIVER), '0', str(source), 'emit-cranelift-simd', str(c), str(obj),
               '--fp-contract', '--algebraic-powers']
    if key.endswith('baseline'):
        command += ['--output', 'energy']
    if extra_flag:
        command.append(extra_flag)
    result = subprocess.run(command, text=True, capture_output=True, check=True)
    (HERE / (key + '.generation.log')).write_text(result.stdout + result.stderr)
    link = [str(CLANG), '-O3', '-isysroot', SDK, str(c), str(obj), '-lm', '-o', str(binary)]
    result = subprocess.run(link, text=True, capture_output=True, check=True)
    (HERE / (key + '.link.log')).write_text(result.stdout + result.stderr)
    record = dict(source=str(source), normalized_loc=expected_loc, source_sha256=sha(source),
                  object_sha256=sha(obj), binary_sha256=sha(binary), generation=command, link=link)
    if frozen_name:
        frozen = FROZEN / frozen_name
        if not frozen.exists():
            candidates = list(FROZEN.glob('*'))
            raise FileNotFoundError((frozen, candidates))
        record['frozen_object_sha256'] = sha(frozen)
        assert record['object_sha256'] == record['frozen_object_sha256'], key
    report['programs'][key] = record
    print('Built', key, expected_loc, 'LOC', record['object_sha256'], flush=True)

def state(binary, n):
    result = subprocess.run([str(binary), str(n)], env=dict(os.environ, MECH_DUMP_STATE='1'),
                            text=True, capture_output=True, check=True)
    values = json.loads(next(line[6:] for line in result.stderr.splitlines() if line.startswith('STATE ')))
    return result.stdout, values

for key in ('compact', 'optimized'):
    previous = key + '-baseline'
    report['programs'][key]['same_object_as_baseline'] = (
        report['programs'][key]['object_sha256'] == report['programs'][previous]['object_sha256'])
    report['programs'][key]['same_generated_runner_as_baseline'] = (
        (HERE / (key + '.c')).read_bytes() == (HERE / (previous + '.c')).read_bytes())
    for n in (0, 1, 10, 1000, 10000, 1_000_000, 50_000_000):
        actual_output, actual = state(HERE / key, n)
        expected_output, expected = state(HERE / previous, n)
        assert actual_output == expected_output, (key, n, actual_output, expected_output)
        assert len(actual) == len(expected) == 30
        bits_equal = all(struct.pack('d', a) == struct.pack('d', b) for a, b in zip(actual, expected))
        assert bits_equal, (key, n, max(abs(a-b) for a, b in zip(actual, expected)))
        report['checks'].append(dict(program=key, steps=n, state_components=30,
                                     bit_identical=True, output=actual_output.strip().splitlines()))
    snapshot = COMPARISON / 'sources' / Path(report['programs'][key]['source']).name
    executable = COMPARISON / 'binaries' / ('mech-matrix-' + key)
    shutil.copy2(report['programs'][key]['source'], snapshot)
    shutil.copy2(HERE / key, executable)
    print(key, 'matches all 30 state components through 50 million steps;',
          'same object:', report['programs'][key]['same_object_as_baseline'], flush=True)
report['status'] = 'complete'
(HERE / 'source-compaction.json').write_text(json.dumps(report, indent=2) + '\n')
