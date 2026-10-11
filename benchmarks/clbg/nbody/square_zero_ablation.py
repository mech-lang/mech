#!/usr/bin/env python3
"""Compare archived square-plus-zero simplification binaries with the current compiler.

Prepare square-zero-sqrt-fma and square-zero-pairwise-fma in
.build/square-zero-comparison with the compiler snapshot named below, then run
this script with the current compiler. Only the latter binaries are rebuilt.
"""
import hashlib
import json
import os
import random
import statistics
import time
from datetime import datetime, timezone
from pathlib import Path
from chapel_tricks import loop_codegen
from run import HERE, command, compiler_snapshot, compiler_sources, metrics, validate


def main():
    steps, samples = 50_000_000, 5
    output = HERE / 'results/apple-m3-square-zero-ablation-50m.json'
    build = HERE / '.build/square-zero-comparison'
    snapshot = compiler_snapshot(output)
    enabled_snapshot = HERE / 'results/apple-m3-square-zero-enabled-50m.compiler.tar.gz'
    command(['cargo', 'build', '--release', '--locked', '--manifest-path', str(HERE / 'mech-driver/Cargo.toml')])
    driver = HERE / 'mech-driver/target/release/mech-clbg-nbody-compiled'
    sdk = command(['xcrun', '--show-sdk-path']).stdout.strip()
    clang = '/opt/homebrew/opt/llvm@22/bin/clang-22'
    rows = []
    for kind, source in [('sqrt-fma', 'nbody.mec'), ('pairwise-fma', 'nbody-pairwise.mec')]:
        binary = build / ('baseline-' + kind)
        generation = [str(driver), '0', str(HERE / source), 'emit-cranelift-simd', str(binary) + '.c', str(binary) + '.o', '--output', 'energy', '--fp-contract', '--algebraic-powers']
        compilation = [clang, '-O3', '-isysroot', sdk, str(binary) + '.c', str(binary) + '.o', '-lm', '-o', str(binary)]
        command(generation)
        command(compilation)
        for variant in ['baseline', 'square-zero']:
            path = build / (variant + '-' + kind)
            row = {'id': variant + '-' + kind, 'command': [str(path), str(steps)], 'source': metrics([source]),
                   'binary_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                   'native_object_sha256': hashlib.sha256(Path(str(path) + '.o').read_bytes()).hexdigest(),
                   'compiler_snapshot': snapshot if variant == 'baseline' else {'path': enabled_snapshot.name, 'sha256': hashlib.sha256(enabled_snapshot.read_bytes()).hexdigest()},
                   'generation_command_template': generation, 'compile_command_template': compilation,
                   'codegen': loop_codegen(path, output.with_name(output.stem + '.' + variant + '-' + kind + '.disassembly.txt'))}
            rows.append(row)
    for language in ['rust', 'chapel']:
        binary = HERE / ('.build/chapel-tricks-' + language + '-reference')
        rows.append({'id': language + '-3', 'command': [str(binary), f'--n={steps}' if language == 'chapel' else str(steps)],
                     'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                     'provenance': 'Unchanged source and build commands in apple-m3-chapel-tricks-square-zero-pilot-5m.json'})
    environment = {**os.environ, 'MECH_TIMINGS': '1', 'OMP_NUM_THREADS': '1', 'CHPL_RT_NUM_THREADS_PER_LOCALE': '1'}
    expected = [-0.169075164, -0.169059907]
    for row in rows:
        validate(command(row['command'], env=environment).stdout, expected)
        row['samples_seconds'], row['samples_stdout'], row['timing_diagnostics'] = [], [], []
    rng = random.Random(20261008)
    order_log = []
    for sample in range(samples):
        order = rows.copy()
        rng.shuffle(order)
        order_log.append([row['id'] for row in order])
        for row in order:
            print(f"Sample {sample+1}/{samples} {row['id']}", flush=True)
            start = time.perf_counter_ns()
            result = command(row['command'], env=environment)
            elapsed = (time.perf_counter_ns() - start) / 1e9
            validate(result.stdout, expected)
            row['samples_seconds'].append(elapsed)
            row['samples_stdout'].append(result.stdout)
            row['timing_diagnostics'].extend(json.loads(line[7:]) for line in result.stderr.splitlines() if line.startswith('TIMING '))
    for row in rows:
        row['median_seconds'] = statistics.median(row['samples_seconds'])
        if row['timing_diagnostics']:
            row['median_kernel_seconds'] = statistics.median(t['kernel_seconds'] for t in row['timing_diagnostics'])
    report = {'created_at': datetime.now(timezone.utc).isoformat(), 'steps': steps, 'samples': samples,
              'hardware': {'cpu': command(['sysctl', '-n', 'machdep.cpu.brand_string']).stdout.strip()},
              'compiler_snapshot': snapshot, 'compiler_sha256': compiler_sources(), 'sample_order': order_log,
              'protocol': 'One discarded warmup, shuffled sequential fresh-process samples, AOT builds excluded; every energy checked. Same pinned binaries within a run.',
              'programs': rows}
    output.write_text(json.dumps(report, indent=2) + '\n')
    for row in rows:
        print(row['id'], row['median_seconds'], row.get('median_kernel_seconds'), flush=True)


if __name__ == '__main__':
    main()
