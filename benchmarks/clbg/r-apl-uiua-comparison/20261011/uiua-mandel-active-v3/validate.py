#!/usr/bin/env python3
"""Locked complete bitmaps, every active recurrence and general array probes."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
SOURCE = HERE / 'sources/fast.ua'
ORIGINAL = SUITE / 'uiua-mandelbrot-v2/sources/fast.ua'
RESULTS = HERE / 'results'
BASE = json.loads((SUITE / 'uiua-mandelbrot-v2/manifest.json').read_text())
RUNTIME = Path(BASE['runtime']['path'])
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
LOCK = Path('/tmp/mech-clbg-timing.lock')
CASES = (1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 200, 257)
TRACE_CASES = (1, 9, 17, 33, 65, 129)
PACK_CASES = (1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 33, 65, 129, 8001)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def f64bytes(values):
    return b''.join(struct.pack('<d', float(value)) for value in values)


def pack(bits):
    padded = bits + [0]*((-len(bits)) % 8)
    return [sum(bit*(1 << (7-j)) for j, bit in enumerate(padded[i:i+8]))
            for i in range(0, len(padded), 8)]


def reference(n):
    rows = []
    used = []
    for y in range(n):
        bits = []
        steps = []
        for x in range(n):
            cx = 2*x/n-1.5
            cy = 2*y/n-1
            r = i = 0.0
            live = True
            count = 0
            for _ in range(50):
                r, i = cx+(r*r-i*i), cy+2*(r*i)
                count += 1
                if r*r+i*i > 4:
                    live = False
                    break
            bits.append(int(live))
            steps.append(count)
        rows.append(pack(bits))
        used.append(steps)
    return f'P4\n{n} {n}\n'.encode()+bytes(byte for row in rows for byte in row), rows, used


def execute(source, n):
    command = [str(RUNTIME), 'run', '--no-format', str(source), str(n)]
    done = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENV), capture_output=True, timeout=120)
    return command, done


def persist(source, n, command, done, stem):
    output = RESULTS / (stem+'.stdout')
    error = RESULTS / (stem+'.stderr')
    output.write_bytes(done.stdout)
    error.write_bytes(done.stderr)
    return dict(source=str(source), source_sha256=sha(source), n=n, command=command,
                cwd=str(SUITE), runtime_environment=ENV, returncode=done.returncode,
                bytes=len(done.stdout), sha256=sha(output), stdout_file=str(output),
                stderr_file=str(error), stderr_sha256=sha(error))


def instrument(source):
    lines = ['# Experimental!']
    for line in source.read_text().splitlines():
        if line.startswith('Advance ←'):
            lines.extend(['Advance ← (', '  '+line.split('←', 1)[1].strip(),
                          '  &p ⊂ "ADVANCED=" json .',
                          '  &p ⊂ "R_BYTES=" json ♭ bytes⌞ "f64" M.R .',
                          '  &p ⊂ "I_BYTES=" json ♭ bytes⌞ "f64" M.I .', ')'])
        elif line.startswith('Iterate ←'):
            lines.extend(['Iterate ← (', '  &p ⊂ "INPUT=" json .',
                          '  '+line.split('←', 1)[1].strip(), '  &p ⊂ "OUT=" json .', ')'])
        else:
            lines.append(line.replace('&fwa "/dev/stdout" Pack', '&p ⊂ "ROW=" json Pack'))
    return '\n'.join(lines)+'\n'


def assert_state(observed, expected):
    assert len(observed) == 6
    r, i, indices, cx, cy, iteration = observed
    er, ei, eindices, ecx, ecy, eiteration = expected
    assert len(r) == len(i) == len(indices) == len(cx)
    assert r == er and i == ei, (r, er, i, ei)
    assert indices == eindices and cx == ecx and cy == ecy and iteration == eiteration
    assert indices == sorted(set(indices))


def check_trace(done, n, wanted_rows, wanted_steps):
    assert done.returncode == 0, done.stderr.decode()
    y = 0
    state = None
    advanced = None
    phase = 'INPUT'
    pixel_counts = [0]*n
    actual_work = 0
    rounds = []
    row_rounds = 0
    observed_rows = []
    for line in done.stdout.decode().splitlines():
        if line in ('P4', f'{n} {n}'):
            continue
        tag, encoded = line.split('=', 1)
        observed = json.loads(encoded)
        if tag == 'ROW':
            assert phase == 'INPUT' and state is not None
            assert state[5] == 50 or not state[2]
            assert pixel_counts == wanted_steps[y], (n, y, pixel_counts, wanted_steps[y])
            bits = [int(x in state[2]) for x in range(n)]
            assert observed == pack(bits) == wanted_rows[y]
            observed_rows.append(observed)
            rounds.append(row_rounds)
            y += 1
            state = None
            pixel_counts = [0]*n
            row_rounds = 0
            continue
        assert tag == phase, (n, y, phase, tag)
        if tag == 'INPUT':
            if state is None:
                state = ([0.0]*n, [0.0]*n, list(range(n)),
                         [(x/n)*2-1.5 for x in range(n)], (y/n)*2-1, 0)
            assert_state(observed, state)
            r, i, indices, cx, cy, iteration = state
            assert iteration < 50 and indices
            ar = [c+(real*real-imag*imag) for c, real, imag in zip(cx, r, i)]
            ai = [cy+2*(real*imag) for real, imag in zip(r, i)]
            advanced = (ar, ai, indices, cx, cy, iteration+1)
            for index in indices:
                pixel_counts[index] += 1
            actual_work += len(indices)
            row_rounds += 1
            phase = 'ADVANCED'
        elif tag == 'ADVANCED':
            assert_state(observed, advanced)
            phase = 'R_BYTES'
        elif tag == 'R_BYTES':
            assert bytes(observed) == f64bytes(advanced[0]), (n, y, 'R f64 bits')
            phase = 'I_BYTES'
        elif tag == 'I_BYTES':
            assert bytes(observed) == f64bytes(advanced[1]), (n, y, 'I f64 bits')
            phase = 'OUT'
        elif tag == 'OUT':
            r, i, indices, cx, cy, iteration = advanced
            keep = [j for j, (real, imag) in enumerate(zip(r, i)) if real*real+imag*imag <= 4]
            state = ([r[j] for j in keep], [i[j] for j in keep],
                     [indices[j] for j in keep], [cx[j] for j in keep], cy, iteration)
            assert_state(observed, state)
            advanced = None
            phase = 'INPUT'
    assert y == n and state is None and observed_rows == wanted_rows
    expected_work = sum(sum(row) for row in wanted_steps)
    assert actual_work == expected_work
    return dict(pixels=n*n, rows=n, actual_pixel_recurrences=actual_work,
                expected_pixel_recurrences=expected_work, maximum_pixel_iterations=50,
                every_requested_pixel_initialized=True, every_active_pixel_identity_verified=True,
                every_active_coordinate_verified=True, every_recurrence_state_bitwise_verified=True,
                escape_iteration_states_included=True, no_escaped_pixel_recurrences=True,
                independent_per_pixel_iteration_counts_verified=True, row_iteration_counts=rounds,
                every_packed_byte_equal_reference=True, reference_match=True)


def literal(values):
    return '['+' '.join(str(value).replace('-', '¯') for value in values)+']'


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    pins = {str(path): sha(path) for path in (SOURCE, ORIGINAL)}
    assert pins[str(ORIGINAL)] == '64f2913ec83009b70cc3ac8c4064438732bc1faf4448af198b7052e4cb0aaa4e'
    for path, digest in BASE['runtime']['pins'].items():
        assert sha(path) == digest, path
    trace = RESULTS / 'instrumented.ua'
    trace.write_text(instrument(SOURCE))
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
                  scope='Bounded complete PBMs, all active/escape states, select/scatter and byte-padding probes; no full timing.',
                  source_pins=pins, runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME),
                  runtime_environment=ENV, shared_timing_lock=str(LOCK),
                  validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
                  cases=[], baseline_cases=[], instrumentation=[], prune_cases=[], scatter_cases=[], primitive_cases=[])
    report_path = RESULTS / 'validation.json'
    try:
        print('Waiting for common mutex; every runtime/reference probe requires acquisition.', flush=True)
        with LOCK.open('a') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            print('Locked bounded active-pixel validation.', flush=True)
            expected = {n: reference(n) for n in CASES}
            for n in CASES:
                command, done = execute(SOURCE, n)
                row = persist(SOURCE, n, command, done, 'output-'+str(n))
                report['cases'].append(row)
                assert done.returncode == 0 and done.stdout == expected[n][0], (n, done.stderr.decode())
                row.update(reference_match=True, pixels=n*n, maximum_pixel_iterations=50,
                           expected_sha256=hashlib.sha256(expected[n][0]).hexdigest(),
                           case='canonical-small' if n == 200 else 'width-'+str(n))
                print(n, 'OUTPUT PASS', flush=True)
            for n in (1, 9, 200, 257):
                command, done = execute(ORIGINAL, n)
                row = persist(ORIGINAL, n, command, done, 'original-'+str(n))
                report['baseline_cases'].append(row)
                assert done.returncode == 0 and done.stdout == expected[n][0], (n, done.stderr.decode())
                row.update(reference_match=True, original_preserved=True)
            for n in TRACE_CASES:
                command, done = execute(trace, n)
                row = persist(trace, n, command, done, 'work-'+str(n))
                report['instrumentation'].append(row)
                row.update(check_trace(done, n, expected[n][1], expected[n][2]))
                row.update(original_source=str(SOURCE), original_source_sha256=pins[str(SOURCE)],
                           transformation='Wrap unchanged Advance/Iterate with complete state and explicit f64-byte diagnostics; serialize exact Pack row instead of writing it. All arithmetic, selection and loop conditions retained; trace never timed.')
                print(n, 'ALL PIXEL/ESCAPE STATE PASS', flush=True)
            selection_lines = [line for line in SOURCE.read_text().splitlines()
                               if line.startswith(('~M ', '~Selection ', 'Selected ', 'Prune '))]
            fixtures = [([], [], [], []), ([0.0], [0.0], [3], [-0.75]),
                        ([3.0], [0.0], [4], [0.125]),
                        ([0.0, 3.0, 1.0, -2.0], [0.0, 0.0, 1.0, 1.0], [1, 4, 7, 11], [-1.4, -0.9, -0.2, 0.1]),
                        ([2.0, 2.0000000000000004, 1.9999999999999998], [0.0]*3, [0, 2, 5], [-1.5, -1.25, -0.875]),
                        ([0.5]*61, [0.5]*61, list(range(1, 123, 2)), [0.25]*61),
                        ([3.0]*65, [0.0]*65, list(range(65)), [-0.5]*65),
                        ([0.25 if j % 3 else 4.0 for j in range(257)], [0.0]*257, list(range(257)), [j/257 for j in range(257)])]
            for index, (r, i, indices, cx) in enumerate(fixtures):
                probe = RESULTS / ('prune-'+str(index)+'.ua')
                probe.write_text('\n'.join(selection_lines+['&p json Prune M '+literal(r)+' '+literal(i)+' '+literal(indices)+' '+literal(cx)+' 0.25 7'])+'\n')
                command, done = execute(probe, 0)
                row = persist(probe, 0, command, done, 'prune-'+str(index))
                report['prune_cases'].append(row)
                keep = [j for j, (real, imag) in enumerate(zip(r, i)) if real*real+imag*imag <= 4]
                wanted = ([r[j] for j in keep], [i[j] for j in keep], [indices[j] for j in keep], [cx[j] for j in keep], 0.25, 7)
                assert done.returncode == 0, done.stderr.decode()
                assert_state(json.loads(done.stdout), wanted)
                row.update(operation='generic-native-where-select', input_rows=len(indices), kept_rows=len(keep),
                           stable_original_indices=True, reference_match=True, benchmark_pixel_recurrences=0)
            scatter_lines = [line for line in SOURCE.read_text().splitlines() if line.startswith(('N ', '~M ', 'Bits '))]
            for index, (n, indices) in enumerate(((0, []), (1, []), (1, [0]), (9, [0, 2, 8]),
                                                   (61, [1, 7, 31, 60]), (65, list(range(65))), (8001, list(range(3, 8001, 11))))):
                probe = RESULTS / ('scatter-'+str(index)+'.ua')
                probe.write_text('\n'.join(scatter_lines+['&p json Bits M [] [] '+literal(indices)+' [] 0 0'])+'\n')
                command, done = execute(probe, n)
                row = persist(probe, n, command, done, 'scatter-'+str(index))
                report['scatter_cases'].append(row)
                wanted = [int(j in indices) for j in range(n)]
                assert done.returncode == 0 and json.loads(done.stdout) == wanted, (n, done.stderr.decode())
                row.update(operation='generic-native-under-select-indexed-update', input_width=n, selected_indices=len(indices),
                           exact_width_restored=True, reference_match=True, benchmark_pixel_recurrences=0)
            packing_lines = [line for line in SOURCE.read_text().splitlines() if line.startswith(('N ', 'Pack '))]
            for n in PACK_CASES:
                bits = [int((37*j+13) % 101 < 47) for j in range(n)]
                probe = RESULTS / ('pack-'+str(n)+'.ua')
                probe.write_text('\n'.join(packing_lines+['&p json Pack '+literal(bits)])+'\n')
                command, done = execute(probe, n)
                row = persist(probe, n, command, done, 'pack-'+str(n))
                report['primitive_cases'].append(row)
                assert done.returncode == 0 and json.loads(done.stdout) == pack(bits), (n, done.stderr.decode())
                row.update(operation='unchanged-native-weight-array-rows-reduce', bits=n,
                           bytes_packed=(n+7)//8, tail_padding=(-n) % 8, reference_match=True, benchmark_pixel_recurrences=0)
            for path, digest in pins.items():
                assert sha(path) == digest, path
        report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(), passed=len(report['cases']),
                      baseline_checks_passed=len(report['baseline_cases']), work_checks_passed=len(report['instrumentation']),
                      prune_checks_passed=len(report['prune_cases']), scatter_checks_passed=len(report['scatter_cases']),
                      primitive_checks_passed=len(report['primitive_cases']), full_performance_runs=0)
        report_path.write_text(json.dumps(report, indent=2, ensure_ascii=False)+'\n')
        print('PASS', report['passed'], 'outputs', report['baseline_checks_passed'], 'baseline controls',
              report['work_checks_passed'], 'all-state traces', report['prune_checks_passed'], 'select',
              report['scatter_checks_passed'], 'scatter', report['primitive_checks_passed'], 'packing', flush=True)
    except BaseException as error:
        report.update(status='failed', failed_at=datetime.now(timezone.utc).isoformat(),
                      error=repr(error), full_performance_runs=0)
        report_path.write_text(json.dumps(report, indent=2, ensure_ascii=False)+'\n')
        raise


if __name__ == '__main__':
    main()
