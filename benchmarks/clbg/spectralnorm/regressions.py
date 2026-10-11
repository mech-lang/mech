#!/usr/bin/env python3
"""Check earlier complete Mech programs after general array/backend changes.

Preserves previous executables and compares them with fresh public AOT builds.
Preparation and validation finish before the shuffled, sequential timing batch.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import tempfile
import time

import run as spectral
import importlib.util
spec=importlib.util.spec_from_file_location("clbg_process", Path(__file__).resolve().parents[1]/"process.py")
spectral_process=importlib.util.module_from_spec(spec);spec.loader.exec_module(spectral_process)

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BUILD = HERE / '.build' / 'previous-regressions'


def sha(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def execute(row, env, n=None):
    argv = row['command'].copy()
    if row['benchmark'] == 'mandelbrot':
        argv += [str(n or 16000), '8']
        stdin_path = None
    else:
        stdin_path = ROOT / 'benchmarks/clbg/knucleotide' / (
            'inputs/knucleotide-input-1000.txt' if n == 1000
            else '.build/knucleotide-input25000000.txt')
    environment = env | row['runtime_environment']
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        stdin = stdin_path.open('rb') if stdin_path else None
        try:
            start = time.perf_counter_ns()
            spectral_process.run_checked(argv, cwd=ROOT, env=environment, stdin=stdin,
                           stdout=output, stderr=errors, timeout=300)
            elapsed = (time.perf_counter_ns() - start) / 1e9
        finally:
            if stdin:
                stdin.close()
        output.seek(0)
        digest = hashlib.file_digest(output, 'sha256').hexdigest()
    return elapsed, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--reuse-builds', action='store_true')
    parser.add_argument('--samples', type=int, default=5)
    parser.add_argument('--output', type=Path, default=HERE / 'results/shared-backend-regression.json')
    parser.add_argument('--build-dir', type=Path, default=BUILD)
    args = parser.parse_args()
    if args.samples < 3:
        parser.error('at least three samples required')
    build_dir = args.build_dir
    build_dir.mkdir(parents=True, exist_ok=True)
    env = os.environ | {'RAYON_NUM_THREADS': '8'}
    if not args.reuse_builds:
        rows = []
        compiler = ROOT / 'benchmarks/clbg/nbody/mech-driver/target/release/mech'
        for benchmark, size in [('mandelbrot', 16000), ('knucleotide', 25000000)]:
            folder = ROOT / 'benchmarks/clbg' / benchmark
            report_path = folder / f'results/apple-m3-revised-{size}.json'
            report = json.loads(report_path.read_text())
            for previous in report['programs']:
                if previous['language'] != 'mech':
                    continue
                source = folder / previous['path']
                if sha(source) != previous['source']['sha256']:
                    raise ValueError('source changed: ' + str(source))
                runtime = previous.get('runtime_environment', {})
                binary = build_dir / previous['id']
                generation = [str(compiler), 'build', str(source), '--backend',
                              'cranelift', '--offline', '--keep-project',
                              '--no-config', '-o', str(binary)]
                build_env = env | {
                    'CARGO_TARGET_DIR': str(ROOT / 'benchmarks/clbg/nbody/mech-driver/target'),
                    'RUSTFLAGS': '-D warnings',
                    'CC': '/opt/homebrew/opt/llvm@22/bin/clang-22',
                }
                if benchmark == 'knucleotide':
                    build_env |= {
                        'MECH_SEQUENCE_BATCH': str(previous['batch']),
                        'MECH_SEQUENCE_SCHEDULE': previous['schedule'],
                        'MECH_SEQUENCE_PACK8': str(int(previous['pack8'])),
                        'MECH_SEQUENCE_FUSE_ENCODE': str(int(previous['fuse_encode'])),
                        'MECH_SEQUENCE_SIMD': str(int(previous['simd'])),
                    }
                elif previous['id'].endswith('compact'):
                    generation += ['--scalar']
                elif previous.get('loop_fma'):
                    generation += ['--fp-contract-loop']
                print('Prepare ' + previous['id'], flush=True)
                subprocess.run(generation, cwd=ROOT, env=build_env, check=True,
                               stdout=subprocess.DEVNULL)
                expected = previous.get('warmup_image', previous.get('warmup_output'))['sha256']
                common = {
                    'benchmark': benchmark, 'source': previous['source'],
                    'runtime_environment': runtime, 'expected_full_sha256': expected,
                    'previous_report': str(report_path.relative_to(ROOT)),
                }
                rows.append(common | {'id': previous['id'] + '-previous',
                                      'command': previous['command'][:1],
                                      'executable_sha256': sha(previous['command'][0])})
                project = Path(str(binary) + '.project')
                obj = project / ('grid.o' if benchmark == 'mandelbrot' else 'mech-sequence.o')
                rows.append(common | {
                    'id': previous['id'] + '-current', 'command': [str(binary)],
                    'executable_sha256': sha(binary), 'native_object_sha256': sha(obj),
                    'generation_command': generation,
                    'compiler_executable_sha256': sha(compiler),
                    'same_native_object_as_previous': sha(obj) == previous['native_object_sha256'],
                    'generated_project_files': {str(f.relative_to(project)): f.read_text()
                                                for f in project.rglob('*') if f.is_file()
                                                and f.suffix in ('.rs', '.toml', '.lock')},
                    'build_environment': {k: build_env[k] for k in build_env if
                                          k.startswith('MECH_') or k in ('RUSTFLAGS', 'CC', 'CARGO_TARGET_DIR')},
                })
        (build_dir / 'prepared.json').write_text(json.dumps(rows, indent=2) + '\n')
        if args.prepare_only:
            return
    else:
        rows = json.loads((build_dir / 'prepared.json').read_text())
    input_pin = json.loads((ROOT / 'benchmarks/clbg/knucleotide/input25000000.json').read_text())
    if sha(ROOT / 'benchmarks/clbg/knucleotide/.build/knucleotide-input25000000.txt') != input_pin['sha256']:
        raise ValueError('full FASTA input differs from pin')
    for row in rows:
        if sha(row['command'][0]) != row['executable_sha256']:
            raise ValueError('executable changed: ' + row['id'])
        n = 200 if row['benchmark'] == 'mandelbrot' else 1000
        small = ROOT / 'benchmarks/clbg' / row['benchmark'] / (
            'upstream/mandelbrot-200.pbm' if n == 200
            else 'inputs/knucleotide-output-1000.txt')
        _, digest = execute(row, env, n)
        if digest != sha(small):
            raise ValueError('official small output differs: ' + row['id'])
        _, digest = execute(row, env)
        if digest != row['expected_full_sha256']:
            raise ValueError('full output differs: ' + row['id'])
        row['correctness_small_and_full'] = True
        row['samples_seconds'] = []
    rng = random.Random(20261007)
    orders = []
    for sample in range(args.samples):
        order = rows.copy()
        rng.shuffle(order)
        orders.append([r['id'] for r in order])
        for row in order:
            elapsed, digest = execute(row, env)
            if digest != row['expected_full_sha256']:
                raise ValueError('timed output differs: ' + row['id'])
            row['samples_seconds'].append(elapsed)
            print(f"{sample + 1}/{args.samples} {row['id']}: {elapsed:.6f}s", flush=True)
    for row in rows:
        row['median_seconds'] = statistics.median(row['samples_seconds'])
    output = args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        'created_at': datetime.now(timezone.utc).isoformat(),
        'hardware': {'platform': platform.platform(), 'machine': platform.machine(),
                     'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip()},
        'protocol': {'samples': args.samples, 'sample_order': orders,
                     'runtime_environment': {'RAYON_NUM_THREADS': '8'},
                     'timed_region': 'Sequential fresh process through exit; complete input and output to regular file. Builds, validation and output hashing excluded.',
                     'comparison': 'Preserved previous executable versus current compiler build of the identical complete Mech source, with identical runtime settings.'},
        'compiler_snapshot': spectral.snapshot(output),
        'programs': rows,
    }
    output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
