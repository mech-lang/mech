#!/usr/bin/env python3
"""Mutexed complete-state and denominator-placement checks; no 50M timing."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
BASE_DIR = SUITE / 'nbody-canonical-v2'
RESULTS = HERE / 'results'
LOCK = Path('/tmp/mech-clbg-timing.lock')
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
CASES = (0, 1, 2, 7, 1000)
VARIANTS = ('compact', 'fast')
RUNTIME_META = json.loads((SUITE / 'uiua-manifest.json').read_text())['runtime']
RUNTIME = Path(RUNTIME_META['path'])
POSITIONS = [[0., 0., 0.], [4.84143144246472090, -1.16032004402742839, -.103622044471123109],
             [8.34336671824457987, 4.12479856412430479, -.403523417114321381],
             [12.8943695621391310, -15.1111514016986312, -.223307578892655734],
             [15.3796971148509165, -25.9193146099879641, .179258772950371181]]
RAW_VELOCITIES = [[0., 0., 0.], [.00166007664274403694, .00769901118419740425, -.0000690460016972063023],
                  [-.00276742510726862411, .00499852801234917238, .0000230417297573763929],
                  [.00296460137564761618, .00237847173959480950, -.0000296589568540237556],
                  [.00268067772490389322, .00162824170038242295, -.0000951592254519715870]]
MASS_RATIOS = [1., .000954791938424326609, .000285885980666130812,
               .0000436624404335156298, .0000515138902046611451]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def left_sum(values):
    result = 0.0
    for value in values:
        result += value
    return result


def binary(values):
    def flatten(value):
        if isinstance(value, list):
            for x in value:
                yield from flatten(x)
        else:
            yield float(value)
    return b''.join(struct.pack('<d', value) for value in flatten(values))


def reference(n):
    """Independent canonical ten-pair integrator, explicit binary64 left folds."""
    solar = 4 * (math.pi * math.pi)
    masses = [solar * ratio for ratio in MASS_RATIOS]
    p = [row.copy() for row in POSITIONS]
    v = [[365.24 * value for value in row] for row in RAW_VELOCITIES]
    v[0] = [-left_sum(masses[i] * v[i][k] for i in range(5)) / solar for k in range(3)]
    initial = [p[i] + v[i] for i in range(5)]
    pairs = [(i, j) for i in range(5) for j in range(i+1, 5)]
    def energy():
        kinetic = left_sum(.5 * masses[i] * left_sum(x*x for x in v[i]) for i in range(5))
        potential = left_sum(masses[i]*masses[j] / math.sqrt(left_sum(
            (p[i][k]-p[j][k]) * (p[i][k]-p[j][k]) for k in range(3))) for i, j in pairs)
        return kinetic - potential
    initial_energy = energy()
    steps = []
    for _ in range(n):
        for i, j in pairs:
            d = [p[i][k]-p[j][k] for k in range(3)]
            squared = left_sum(x*x for x in d)
            magnitude = .01 / (squared * math.sqrt(squared))
            for k in range(3):
                v[i][k] -= d[k]*masses[j]*magnitude
                v[j][k] += d[k]*masses[i]*magnitude
        p = [[p[i][k]+.01*v[i][k] for k in range(3)] for i in range(5)]
        steps.append([p[i]+v[i] for i in range(5)])
    return dict(initial=initial, final=[p[i]+v[i] for i in range(5)], steps=steps,
                masses=masses, velocities=[[365.24*x for x in row] for row in RAW_VELOCITIES],
                energies=[initial_energy, energy()])


def instrument(source):
    lines = ['# Experimental!']
    for line in source.read_text().splitlines():
        if line.startswith('Advance      ←'):
            lines.extend(['Advance ← (', '  ' + line.split('←', 1)[1].strip(),
                          '  &p ⊂ "STEP_BYTES=" json ♭ bytes⌞ "f64" .', ')'])
        elif line == 'Print ⍥Advance N Initial':
            lines.extend(['Final ← ⍥Advance N Initial', 'Print Final',
                          '&p ⊂ "STATE=" json Final',
                          '&p ⊂ "STATE_BYTES=" json ♭ bytes⌞ "f64" Final',
                          '&p ⊂ "POSITIONS_BYTES=" json ♭ bytes⌞ "f64" Positions',
                          '&p ⊂ "VELOCITIES_BYTES=" json ♭ bytes⌞ "f64" Velocities',
                          '&p ⊂ "MASSES_BYTES=" json ♭ bytes⌞ "f64" Masses'])
        else:
            lines.append(line)
    assert any(line.startswith('Final ←') for line in lines)
    return '\n'.join(lines) + '\n'


def execute(source, n, data=None):
    command = [str(RUNTIME), 'run', '--no-format', str(source), str(n)]
    done = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENV),
                          input=data, capture_output=True, timeout=120)
    return command, done


def persist(source, n, command, done, stem):
    out = RESULTS / (stem + '.stdout')
    err = RESULTS / (stem + '.stderr')
    out.write_bytes(done.stdout)
    err.write_bytes(done.stderr)
    return dict(source=str(source), source_sha256=sha(source), n=n, command=command,
                cwd=str(SUITE), runtime_environment=ENV, returncode=done.returncode,
                bytes=len(done.stdout), sha256=sha(out), stdout_file=str(out),
                stderr_file=str(err), stderr_sha256=sha(err))


def parse_trace(done, n):
    assert done.returncode == 0, done.stderr.decode()
    record = dict(steps=[], energies=[])
    for line in done.stdout.decode().splitlines():
        if line.startswith('STEP_BYTES='):
            record['steps'].append(bytes(json.loads(line[len('STEP_BYTES='):])) )
        elif '=' in line:
            key, value = line.split('=', 1)
            data = json.loads(value)
            record[key.lower()] = bytes(data) if key.endswith('_BYTES') else data
        else:
            record['energies'].append(float(line))
    assert len(record['steps']) == n
    assert len(record['state_bytes']) == 30*8
    assert len(record['state']) == 5 and all(len(row) == 6 for row in record['state'])
    assert all(len(step) == 30*8 for step in record['steps'])
    assert all(math.isfinite(x) for x in struct.unpack('<30d', record['state_bytes']))
    assert all(math.isfinite(x) for step in record['steps'] for x in struct.unpack('<30d', step))
    if n:
        assert record['steps'][-1] == record['state_bytes']
    return record


def state_error(encoded, expected):
    observed = struct.unpack('<30d', encoded)
    wanted = struct.unpack('<30d', binary(expected))
    return max(abs(a-b) for a, b in zip(observed, wanted))


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    sources = {variant: HERE / 'sources' / (variant+'.ua') for variant in VARIANTS}
    originals = {variant: BASE_DIR / 'sources/uiua' / (variant+'.ua') for variant in VARIANTS}
    pins = {str(path): sha(path) for path in [*sources.values(), *originals.values()]}
    for path, digest in RUNTIME_META['pins'].items():
        assert sha(path) == digest, path
    traces = {}
    for variant in VARIANTS:
        for group, mapping in [('original', originals), ('candidate', sources)]:
            path = RESULTS / (group+'-'+variant+'-trace.ua')
            path.write_text(instrument(mapping[variant]))
            traces[group, variant] = path
    original_wrong = SUITE / 'sources/uiua/nbody/fast.ua'
    z_correct = -.103622044471123109
    z_wrong = -.10362204447112314
    bits = lambda x: struct.unpack('<Q', struct.pack('<d', x))[0]
    assert abs(bits(z_correct)-bits(z_wrong)) == 2
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
                  scope='Canonical small outputs, every 30-lane state at n0/1/2/7/1000 and generic denominator matrices only. No 50M timing.',
                  runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME), runtime_environment=ENV,
                  source_pins=pins, shared_timing_lock=str(LOCK), validator=str(Path(__file__).resolve()),
                  validator_sha256=sha(__file__), reference='Independent canonical ten-pair binary64 integrator with explicit left folds.',
                  reference_source=str(SUITE / 'upstream/apl/nbody.reference.d'),
                  reference_source_sha256=sha(SUITE / 'upstream/apl/nbody.reference.d'),
                  canonical_jupiter_z=z_correct, canonical_jupiter_z_hex=z_correct.hex(),
                  original_wrong_source=str(original_wrong), original_wrong_source_sha256=sha(original_wrong),
                  original_wrong_jupiter_z=z_wrong, original_wrong_jupiter_z_hex=z_wrong.hex(),
                  canonical_correction_ulps=2, cases=[], state_cases=[], primitive_cases=[])
    print('Waiting for common mutex; all reference/runtime probes occur after acquisition.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Locked bounded full-state denominator validation.', flush=True)
        expected = {n: reference(n) for n in CASES}
        for variant in VARIANTS:
            for n in CASES:
                command, done = execute(sources[variant], n)
                row = persist(sources[variant], n, command, done, variant+'-'+str(n))
                assert done.returncode == 0, done.stderr.decode()
                energies = [float(line) for line in done.stdout.decode().splitlines()]
                assert len(energies) == 2 and all(math.isfinite(x) for x in energies)
                error = max(abs(a-b) for a, b in zip(energies, expected[n]['energies']))
                assert error <= 1e-8, (variant, n, energies, expected[n]['energies'])
                row.update(variant=variant, case='canonical-small' if n == 1000 else 'canonical-'+str(n),
                           energies=energies, reference_energies=expected[n]['energies'],
                           energy_tolerance=1e-8, maximum_energy_absolute_error=error, reference_match=True)
                report['cases'].append(row)
                state_records = {}
                for group in ('original', 'candidate'):
                    trace = traces[group, variant]
                    trace_command, trace_done = execute(trace, n)
                    data = parse_trace(trace_done, n)
                    evidence = persist(trace, n, trace_command, trace_done, group+'-'+variant+'-state-'+str(n))
                    state_records[group] = data, evidence
                old_data, old_evidence = state_records['original']
                new_data, new_evidence = state_records['candidate']
                assert new_data['state_bytes'] == old_data['state_bytes']
                assert new_data['steps'] == old_data['steps']
                assert new_data['positions_bytes'] == binary(POSITIONS)
                assert new_data['velocities_bytes'] == binary(expected[n]['velocities'])
                assert new_data['masses_bytes'] == binary(expected[n]['masses'])
                reference_error = state_error(new_data['state_bytes'], expected[n]['final'])
                assert reference_error <= 5e-11, (variant, n, reference_error)
                step_error = max((state_error(step, state) for step, state in zip(new_data['steps'], expected[n]['steps'])), default=0.0)
                assert step_error <= 5e-11, (variant, n, step_error)
                if n == 0:
                    assert new_data['state_bytes'] == binary(expected[n]['initial'])
                assert new_data['energies'] == energies == old_data['energies']
                new_evidence.update(variant=variant, original_source=str(sources[variant]),
                                    original_source_sha256=sha(sources[variant]), canonical_baseline_source=str(originals[variant]),
                                    canonical_baseline_source_sha256=sha(originals[variant]), baseline_evidence=old_evidence,
                                    transformation='Wrap each unchanged Advance result with explicit f64 little-endian byte reporting; report final state and raw constants. Experimental mode is diagnostic only.',
                                    bodies=5, state_shape=[5, 6], state_components=30, verified_steps=n,
                                    final_state_bitwise_equal=True, every_step_bitwise_equal=True,
                                    canonical_raw_constants_bitwise_equal=True,
                                    canonical_initial_state_bitwise_equal=n == 0,
                                    canonical_zero_jupiter_z_ulp_difference=0,
                                    independent_pairwise_state_tolerance=5e-11,
                                    maximum_final_reference_absolute_error=reference_error,
                                    maximum_step_reference_absolute_error=step_error, reference_match=True)
                report['state_cases'].append(new_evidence)
                print(variant, n, 'OUTPUT + 30-LANE STATE PASS', flush=True)
        primitive = RESULTS / 'denominator-only.ua'
        primitive.write_text('# Experimental!\nX ← ≡°□ °json &fras "/dev/stdin"\nOld ← ≡≡(¤√ⁿ3)\nNew ← ≡≡¤ √ⁿ3\n&p json ⊟⊃(Old)(New) X\n')
        for rows, columns in ((1, 1), (2, 3), (3, 2), (5, 5), (7, 4), (1, 8), (9, 1)):
            data = [[.25 + ((17*i+23*j+5)%67)/8 for j in range(columns)] for i in range(rows)]
            input_bytes = json.dumps(data).encode()
            command, done = execute(primitive, 0, input_bytes)
            assert done.returncode == 0, done.stderr.decode()
            old_values, new_values = json.loads(done.stdout)
            wanted = [[[math.sqrt((value*value)*value)] for value in row] for row in data]
            assert old_values == new_values == wanted
            assert binary(old_values) == binary(new_values) == binary(wanted)
            row = persist(primitive, 0, command, done, 'denominator-'+str(rows)+'x'+str(columns))
            inp = RESULTS / ('denominator-'+str(rows)+'x'+str(columns)+'.input.json')
            inp.write_bytes(input_bytes)
            row.update(operation='native-pervasive-power-sqrt-depth-fix', input_file=str(inp), input_sha256=sha(inp),
                       input_shape=[rows, columns], output_shape=[rows, columns, 1],
                       same_values_and_order=True, reference_match=True, benchmark_steps=0)
            report['primitive_cases'].append(row)
        for path, digest in pins.items():
            assert sha(path) == digest, path
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                  passed=len(report['cases']), state_checks_passed=len(report['state_cases']),
                  primitive_checks_passed=len(report['primitive_cases']), full_performance_runs=0)
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False)+'\n')
    print('PASS', report['passed'], 'outputs', report['state_checks_passed'],
          'full-state comparisons', report['primitive_checks_passed'], 'generic denominator probes', flush=True)


if __name__ == '__main__':
    main()
