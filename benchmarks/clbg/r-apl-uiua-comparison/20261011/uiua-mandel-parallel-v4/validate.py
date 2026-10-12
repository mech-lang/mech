#!/usr/bin/env python3
"""Mutexed complete PBMs, ordered worker-returned traces and native worker gates."""
from datetime import datetime, timezone
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import time

from prepare import BASES, HERE, LOCK, SUITE, prepare, sha
from task_threads import TaskThreads

RESULTS = HERE / 'results'
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1',
           RAYON_NUM_THREADS='1')
POOL_GATE_INIT = '◌ ⍏ ◿16001 ×7919 ⇡16000'
CASES = (1, 2, 7, 8, 9, 13, 31, 63, 64, 65, 129, 200, 257)
TRACE_CASES = (9, 13, 65)
GATE_CASES = (1, 2, 7, 8, 9, 13, 65)


def save(path, value):
    temporary = path.with_suffix(path.suffix+'.tmp')
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False)+'\n')
    temporary.replace(path)


def persist(source, n, command, status, output, error, observations, stem):
    stdout = RESULTS / (stem+'.stdout')
    stderr = RESULTS / (stem+'.stderr')
    stdout.write_bytes(output)
    stderr.write_bytes(error)
    return dict(source=str(source), source_sha256=sha(source), n=n, command=command,
                cwd=str(SUITE), runtime_environment=ENV, returncode=status,
                bytes=len(output), stdout_file=str(stdout), stdout_sha256=sha(stdout),
                stderr_file=str(stderr), stderr_sha256=sha(stderr),
                native_thread_observations=observations,
                peak_total_os_threads=max((row['total_os_threads'] for row in observations), default=0))


def execute(runtime, source, n, tasks, gate=False):
    command = [str(runtime), 'run', '--no-format', str(source), str(n)]
    outpath = RESULTS / 'capture.stdout.tmp'
    errpath = RESULTS / 'capture.stderr.tmp'
    observations = []
    blocked = []
    with outpath.open('w+b') as output, errpath.open('w+b') as error:
        proc = subprocess.Popen(command, cwd=SUITE, env=dict(os.environ, **ENV),
                                stdin=subprocess.PIPE if gate else subprocess.DEVNULL,
                                stdout=output, stderr=error, start_new_session=True)
        deadline = time.monotonic()+120
        released = not gate
        try:
            while proc.poll() is None:
                assert time.monotonic() < deadline, 'Bounded validation timed out'
                observed = tasks.read(proc.pid)
                if observed:
                    observations.append(observed)
                if gate and not released:
                    output.flush()
                    output.seek(0)
                    if output.read(6) == b'READY\n':
                        for _ in range(3):
                            value = tasks.read(proc.pid)
                            assert value is not None
                            blocked.append(value)
                            time.sleep(.005)
                        proc.stdin.write(b'release\n')
                        proc.stdin.close()
                        released = True
                time.sleep(.005)
            assert released, 'Worker gate never reached READY'
        except BaseException:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
            raise
        status = proc.wait()
        output.seek(0)
        error.seek(0)
        return command, status, output.read(), error.read(), observations, blocked


def traced_source(source, family):
    text = source.read_text()
    if family == 'whole':
        ids = '⊚M.Live'
        real = '⊏⊃(⊚M.Live)M.R'
        imag = '⊏⊃(⊚M.Live)M.I'
        coordinates = '⊏⊃(⊚M.Live)M.Cx'
        finish = 'Mark'
        condition = '×⊃(<50M.Iter Recorder.State)(/↥M.Live Recorder.State)'
        bits = 'M.Live'
    else:
        ids, real, imag, coordinates = 'M.Index', 'M.R', 'M.I', 'M.Cx'
        finish = 'Prune'
        condition = '×⊃(<50M.Iter Recorder.State)(>0⧻M.Index Recorder.State)'
        bits = 'Bits'
    helpers = [
        '~Frame {Ids Cy Iter Rbytes Ibytes Cx}',
        'FrameOf ← Frame ⊃('+ids+'|M.Cy|M.Iter|♭ bytes⌞ "f64" '+real+'|♭ bytes⌞ "f64" '+imag+'|'+coordinates+')',
        '~Recorder {State Frames}',
        'ToAdvance ← Recorder ⊃(Advance Recorder.State|Recorder.Frames)',
        'Capture ← Recorder ⊃('+finish+' Recorder.State|⊂⊃Recorder.Frames(□ FrameOf Recorder.State))',
        'Logged ← Capture ToAdvance',
        '~TracedRow {Packed Frames}',
    ]
    lines = ['# Experimental!']
    for line in text.splitlines():
        if line.startswith('Row '):
            initial = line.split(')) ', 1)[1]
            lines += helpers
            lines.append('Row ← TracedRow ⊃(Pack '+bits+' Recorder.State|Recorder.Frames) ⍢(Logged|'+condition+') Recorder ⊃∘(⋅[]) '+initial)
        elif line.startswith('&p '):
            continue
        elif line.startswith('&fwa '):
            lines.append(line.replace('&fwa "/dev/stdout"', '&p json'))
        else:
            lines.append(line)
    return '\n'.join(lines)+'\n'


def gate_source(source):
    lines = [POOL_GATE_INIT]
    for line in source.read_text().splitlines():
        if line.startswith('Worker '):
            lines += ['Worker ← (', '  ◌ recv 0', '  '+line.split('←', 1)[1].strip(), ')']
        else:
            lines.append(line)
        if line.startswith('Handles '):
            lines += ['&p "READY"', '◌ &fras "/dev/stdin"', 'send Handles 1']
    return '\n'.join(lines)+'\n'


def check_trace(output, n, reference, numeric):
    rows = json.loads(output)
    _, expected_rows, expected_counts = reference(n)
    assert len(rows) == n
    work = 0
    for y, (packed, frames) in enumerate(rows):
        assert packed == expected_rows[y]
        r = [0.0]*n
        i = [0.0]*n
        alive = list(range(n))
        counts = [0]*n
        cy = (y/n)*2-1
        for iteration, frame in enumerate(frames, 1):
            ids, observed_cy, observed_iteration, rb, ib, cx = frame
            assert ids == alive and observed_cy == cy and observed_iteration == iteration
            assert cx == [(x/n)*2-1.5 for x in alive]
            nr, ni = [], []
            for x in alive:
                nr.append((x/n)*2-1.5+(r[x]*r[x]-i[x]*i[x]))
                ni.append(cy+2*(r[x]*i[x]))
                counts[x] += 1
            assert bytes(rb) == numeric(nr) and bytes(ib) == numeric(ni), (n, y, iteration)
            next_alive = []
            for x, real, imag in zip(alive, nr, ni):
                r[x], i[x] = real, imag
                if real*real+imag*imag <= 4:
                    next_alive.append(x)
            alive = next_alive
            work += len(ids)
        assert len(frames) == 50 or not alive
        assert counts == expected_counts[y]
        assert packed == expected_rows[y]
    assert work == sum(sum(row) for row in expected_counts)
    return dict(reference_match=True, every_row_order_verified=True, every_requested_pixel_verified=True,
                every_required_recurrence_bitwise_verified=True, escape_states_included=True,
                actual_required_pixel_recurrences=work, worker_returned_trace_only=True,
                concurrent_worker_stdout=False, maximum_pixel_iterations=50)


def main():
    global RESULTS
    RESULTS = RESULTS / ('validation-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ'))
    RESULTS.mkdir(parents=True, exist_ok=True)
    print('Waiting for common mutex before preparation, hashing or runtime/reference probes.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Locked isolated Uiua parallel preparation/validation.', flush=True)
        prepare()
        baseline = json.loads((SUITE / 'uiua-mandel-active-v3/manifest.json').read_text())
        runtime = Path(baseline['runtime']['path'])
        for path, digest in baseline['runtime']['pins'].items():
            assert sha(path) == digest, path
        spec = importlib.util.spec_from_file_location('independent_pixel_reference', SUITE / 'uiua-mandel-active-v3/validate.py')
        reference = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(reference)
        tasks = TaskThreads()
        source_pins = {str(HERE / 'sources' / (family+'.ua')): sha(HERE / 'sources' / (family+'.ua')) for family in BASES}
        source_pins.update({str(path): digest for path, digest, _ in BASES.values()})
        report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(), source_pins=source_pins,
                      shared_timing_lock=str(LOCK), runtime=str(runtime), runtime_sha256=sha(runtime), runtime_environment=ENV,
                      validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
                      reference_validator=str(Path(spec.origin)), reference_validator_sha256=sha(spec.origin),
                      cases=[], baseline_cases=[], traces=[], worker_gates=[], full_performance_runs=0)
        report['attempt_directory'] = str(RESULTS)
        try:
            expected = {n: reference.reference(n)[0] for n in CASES}
            parent_gate = RESULTS / 'parent-only-gate.ua'
            parent_gate.write_text(POOL_GATE_INIT+'\n&p "READY"\n◌ &fras "/dev/stdin"\n')
            cmd, status, output, error, observations, blocked = execute(runtime, parent_gate, 0, tasks, gate=True)
            assert status == 0 and output == b'READY\n'
            parent = persist(parent_gate, 0, cmd, status, output, error, observations, 'parent-only-gate')
            parent['blocked_observations'] = blocked
            baseline_threads = blocked[0]['total_os_threads']
            assert all(row['total_os_threads'] == baseline_threads for row in blocked)
            report['parent_thread_baseline'] = parent
            report['parent_total_os_threads'] = baseline_threads
            for family in BASES:
                source = HERE / 'sources' / (family+'.ua')
                trace = RESULTS / ('trace-'+family+'.ua')
                trace.write_text(traced_source(source, family))
                gate = RESULTS / ('gate-'+family+'.ua')
                gate.write_text(gate_source(source))
                for n in CASES:
                    cmd, status, output, error, observed, _ = execute(runtime, source, n, tasks)
                    row = persist(source, n, cmd, status, output, error, observed, family+'-output-'+str(n))
                    report['cases'].append(row)
                    assert status == 0 and output == expected[n], (family, n, error.decode())
                    assert row['peak_total_os_threads'] <= baseline_threads+min(8, n), row
                    row.update(family=family, reference_match=True, pixels=n*n, spawned_worker_limit=min(8, n),
                               parent_only_output=True, worker_results_joined_before_binary_payload=True,
                               case='canonical-small' if n == 200 else 'width-'+str(n))
                    print(family, n, 'OUTPUT PASS', flush=True)
                for n in (9, 200):
                    base = BASES[family][0]
                    cmd, status, output, error, observed, _ = execute(runtime, base, n, tasks)
                    row = persist(base, n, cmd, status, output, error, observed, family+'-retained-base-'+str(n))
                    report['baseline_cases'].append(row)
                    assert status == 0 and output == expected[n]
                    row.update(family=family, reference_match=True, retained_base_unchanged=True)
                for n in TRACE_CASES:
                    cmd, status, output, error, observed, _ = execute(runtime, trace, n, tasks)
                    row = persist(trace, n, cmd, status, output, error, observed, family+'-trace-'+str(n))
                    report['traces'].append(row)
                    assert status == 0, (family, n, error.decode())
                    row.update(check_trace(output, n, reference.reference, reference.f64bytes))
                    row.update(family=family, original_source=str(source), original_source_sha256=sha(source))
                    print(family, n, 'RETURNED TRACE PASS', flush=True)
                for n in GATE_CASES:
                    cmd, status, output, error, observed, blocked = execute(runtime, gate, n, tasks, gate=True)
                    row = persist(gate, n, cmd, status, output, error, observed, family+'-worker-gate-'+str(n))
                    report['worker_gates'].append(row)
                    assert status == 0 and output == b'READY\n'+expected[n], (family, n, error.decode())
                    assert all(item['total_os_threads']-baseline_threads == min(8, n) for item in blocked), blocked
                    row.update(family=family, reference_match=True, diagnostic_only=True,
                               blocked_observations=blocked, parent_total_os_threads=baseline_threads,
                               observed_additional_os_worker_threads=min(8, n), expected_spawn_workers=min(8, n),
                               actual_worker_bound_verified=True, benchmark_native_runtime_unmodified=True)
                    print(family, n, 'NATIVE WORKER GATE PASS', min(8, n), flush=True)
            for path, digest in source_pins.items():
                assert sha(path) == digest
            report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                          outputs_passed=len(report['cases']), baseline_checks_passed=len(report['baseline_cases']),
                          traces_passed=len(report['traces']), worker_gates_passed=len(report['worker_gates']))
            save(RESULTS / 'validation.json', report)
            save(HERE / 'results/validation.json', report)
            manifest_spec = importlib.util.spec_from_file_location('parallel_manifest', HERE / 'prepare_manifest.py')
            manifest_module = importlib.util.module_from_spec(manifest_spec)
            manifest_spec.loader.exec_module(manifest_module)
            manifest_module.main()
            print('PASS', report['outputs_passed'], 'outputs', report['baseline_checks_passed'], 'retained-base controls',
                  report['traces_passed'], 'returned all-state traces', report['worker_gates_passed'], 'native worker gates', flush=True)
        except BaseException as error:
            report.update(status='failed', failed_at=datetime.now(timezone.utc).isoformat(), error=repr(error))
            save(RESULTS / 'validation.json', report)
            save(HERE / 'results/validation.json', report)
            raise


if __name__ == '__main__':
    main()
