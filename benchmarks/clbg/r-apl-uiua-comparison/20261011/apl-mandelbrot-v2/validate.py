"""Mutexed independent PBM and exact per-pixel iteration-count validation."""
from pathlib import Path
import copy
import datetime
import fcntl
import hashlib
import json
import math
import os
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
G = HERE.parent
sys.path.insert(0, str(G))
from prepare import LOCK, REQUIRED, compact, save, sha, support_pins
from run import PinGuard, kill_group, group_members
sys.path.insert(0, str(HERE))
from prepare_sources import loc


def reference(n):
    output = bytearray(f'P4\n{n} {n}\n'.encode())
    counts = []
    for y in range(n):
        ci = -1.0 + 2.0 * (y / n)
        packed = bytearray((n + 7) // 8)
        for x in range(n):
            cr = -1.5 + 2.0 * (x / n)
            zr = zi = 0.0
            alive = True
            for iteration in range(50):
                r2 = zr * zr
                i2 = zi * zi
                zi = ci + 2.0 * (zr * zi)
                zr = cr + (r2 - i2)
                if zr * zr + zi * zi > 4.0:
                    alive = False
                    break
            counts.append(iteration + 1)
            if alive:
                packed[x // 8] |= 1 << (7 - x % 8)
        output += packed
    return bytes(output), counts


def execute(path, n, trace=None, workers=1):
    command = [str(G / 'run-apl.sh'), '--cc', str(workers), '-f', str(path), '--', str(n)]
    if trace is not None:
        command.append(str(trace))
    process = subprocess.Popen(command, cwd=G, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        env=compact.ENV | {'LC_ALL': 'C'}, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=120)
    except BaseException:
        kill_group(process.pid)
        process.wait()
        raise
    assert process.returncode == 0, (command, stderr.decode(errors='replace'))
    assert not group_members(process.pid), 'Leftover validation process group'
    assert not re.search(rb'(?:DOMAIN|RANK|LENGTH|INDEX|SYNTAX|VALUE|WS FULL) ERROR', stderr), stderr.decode(errors='replace')
    return command, stdout, stderr


def traced(text, variant):
    text = text.replace(';Pad\n', ';Pad;Work\n')
    shape = 'N' if variant == 'compact' else '(Rows×N)'
    text = text.replace(' Iter←0\n', f' Work←{shape}⍴0\n Iter←0\n')
    text = text.replace(' R2←ZR×ZR\n', ' Work[Index]←Work[Index]+1\n R2←ZR×ZR\n')
    # A diagnostic copy adds only counters/output; timed application remains untouched.
    text = text.replace(' ⊣Bytes⎕FIO[7]OUT\n', " ⊣Bytes⎕FIO[7]OUT\n ⊣((⍕Work),⎕UCS 10)⎕FIO[23]TRACE\n")
    text = text.replace('Run ⍎,⊃¯1↑⎕ARG\n', "TRACE←'w'⎕FIO[3],⊃¯1↑⎕ARG\nRun ⍎,⊃1↑¯2↑⎕ARG\n⊣⎕FIO[4]TRACE\n")
    assert text.count('Work[Index]←Work[Index]+1') == 1
    return text


def main():
    preserved = json.loads((HERE / 'preserved.json').read_text())
    for path, digest in preserved['pins'].items():
        assert sha(path) == digest, path
    old = json.loads((G / 'apl-manifest.json').read_text())
    rows = []
    records = []
    sizes = [1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 65, 129, 200, 257]
    results = HERE / 'results'
    results.mkdir(parents=True, exist_ok=True)
    pins = support_pins(old)
    # Historical preparation metadata contains prior manifest hashes under
    # the same logical filename. This validator owns the current manifest
    # identity explicitly, verified above; application/runtime pins stay exact.
    pins.update(preserved['pins'])
    runtime = old['runtime']
    pins[runtime['path']] = runtime['sha256']
    for source in preserved['sources']:
        pins[source['source']] = source['source_sha256']
    guard = PinGuard(pins)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        compact.wait_for_environment(REQUIRED)
        guard.verify()
        for variant in ['compact', 'optimized']:
            path = HERE / 'sources' / ('mandelbrot.' + variant + '.apl')
            source_text = path.read_text()
            trace_source = results / ('mandelbrot.' + variant + '.trace.apl')
            trace_source.write_text(traced(source_text, variant))
            for n in sizes:
                expected, work = reference(n)
                command, output, stderr = execute(path, n, workers=1 if variant == 'compact' else 8)
                assert output == expected, (variant, n, output[:100], expected[:100])
                trace_path = results / (variant + '-' + str(n) + '.work')
                trace_command, trace_output, trace_stderr = execute(trace_source, n, trace=trace_path,
                    workers=1 if variant == 'compact' else 8)
                observed_work = [int(value) for value in trace_path.read_text().split()]
                assert trace_output == expected, (variant, n, 'trace PBM changed')
                assert observed_work == work, (variant, n, 'per-pixel iteration counts differ')
                output_path = results / (variant + '-' + str(n) + '.pbm')
                output_path.write_bytes(output)
                (results / (variant + '-' + str(n) + '.stderr')).write_bytes(stderr)
                record = dict(variant=variant, case='canonical-small' if n == 200 else 'width-' + str(n),
                    n=n, command=command, source=str(path), source_sha256=sha(path),
                    returncode=0, passed=True, reference_match=True,
                    bytes=len(output), sha256=hashlib.sha256(output).hexdigest(), stdout_file=str(output_path),
                    expected_sha256=hashlib.sha256(expected).hexdigest(),
                    work_trace=dict(command=trace_command, source=str(trace_source), source_sha256=sha(trace_source),
                        path=str(trace_path), sha256=sha(trace_path), pixels=n*n,
                        updates=sum(work), reference_match=True))
                records.append(record)
                print('PASS', variant, n, len(output), 'bytes', sum(work), 'updates', flush=True)
            guard.verify()
    validation_path = results / 'validation.json'
    save(validation_path, dict(passed=True, rows=records, checks=len(records)*2,
        reference='Independent scalar binary64 z update and per-pixel escape count for all coordinates; no golden output constants.',
        preserved_pins=preserved['pins'], full_attempts=0))
    for variant in ['compact', 'optimized']:
        prior = next(row for row in old['programs'] if row['benchmark'] == 'mandelbrot' and row['variant'] == variant)
        row = copy.deepcopy(prior)
        path = HERE / 'sources' / ('mandelbrot.' + variant + '.apl')
        row.update(id='mandelbrot-apl-active-v2-' + variant,
            key='mandelbrot:mandelbrot-apl-active-v2-' + variant,
            source=str(path), source_sha256=sha(path), loc=loc(path.read_text()),
            normalized_loc=loc(path.read_text()), physical_loc=loc(path.read_text()), loc_expansions=[],
            command=[str(G / 'run-apl.sh'), '--cc', '1' if variant == 'compact' else '8', '-f', str(path), '--', '{n}'],
            small_validation=next(r for r in records if r['variant'] == variant and r['n'] == 200),
            adversarial_validation=dict(report=str(validation_path), sha256=sha(validation_path),
                passed=True, cases_per_variant=len(sizes), per_pixel_work_trace=True),
            measurement_status='prepared-not-fully-timed', eligible_for_chart=False,
            supersedes_keys=[prior['key']], supersedes_source_sha256=prior['source_sha256'],
            samples_seconds=[], sample_validation=[], attempts=[])
        row['adaptations'] = [
            'Every pixel starts at z=0 and receives the canonical binary64 recurrence until its first squared magnitude>4 or all50 updates.',
            'Maintain a stable vector of active pixel indices; gather coordinate arrays, compress z arrays by the escape mask after each actual update, and persist the final alive mask at original pixel positions.',
            'Process one row at a time.' if variant == 'compact' else 'Process at most16rows at a time, including final partial row blocks.',
            'Pack the complete alive mask into PBM bytes in row order with zero padding for odd widths. All helpers/application statements count.',
            'Requested --cc flag is an interpreter ceiling only. This pinned macOS GNU APL runtime has PARALLEL_ENABLED disabled and executes numerical primitives serially; no runtime build change.']
        row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
            reason='Ordinary APL stable compress, gather, scatter and arithmetic only; every surviving pixel is updated and tested in canonical order, escaped pixels never become alive again. No region/interior tests, symmetry, cached output, approximate math or benchmark host kernel.',
            active_pixel_compression=True, maximum_rows=1 if variant == 'compact' else 16,
            primitive_parallelism='disabled-in-pinned-macos-build')
        for field in ['timing_alias_of', 'full_failure', 'full_validation', 'median_seconds',
                      'sample_stdev_seconds', 'aliased_sample_source']:
            row.pop(field, None)
        rows.append(row)
    save(HERE / 'manifest.json', dict(schema_version=1, status='prepared-validated', language='apl',
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), programs=rows,
        runtime=old['runtime'], validation=dict(path=str(validation_path), sha256=sha(validation_path),
            passed=True, total_checks=len(records)*2), preserved_pins=preserved['pins'],
        source_recipe=dict(path=str(HERE / 'prepare_sources.py'), sha256=sha(HERE / 'prepare_sources.py')),
        policy='Additive counted candidates. Original frozen sources and real measurements remain unchanged; selection/full timing requires approved safe guard revision.'))
    print('MANIFEST', HERE / 'manifest.json', flush=True)


if __name__ == '__main__':
    main()
