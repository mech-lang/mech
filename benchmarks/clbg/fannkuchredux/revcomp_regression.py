#!/usr/bin/env python3
"""Rebuild the unchanged reverse-complement program and compare preserved AOT."""
import argparse, json, os, random, statistics, subprocess
from pathlib import Path
import run

HERE = run.HERE
ROOT = run.ROOT
BUILD = run.BUILD / 'revcomp-regression'
FOLDER = HERE.parent / 'revcomp'
previous_run = run.module('previous_revcomp', FOLDER / 'run.py')

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--prepare-only', action='store_true')
    ap.add_argument('--reuse-builds', action='store_true')
    ap.add_argument('--samples', type=int, default=7)
    args = ap.parse_args()
    if args.samples < 3:
        ap.error('at least three samples required')
    BUILD.mkdir(parents=True, exist_ok=True)
    env = os.environ | {'RAYON_NUM_THREADS': '8'}
    if not args.reuse_builds:
        report = json.loads((FOLDER / 'results/apple-m3-final-25000000.json').read_text())
        old = next(r for r in report['programs'] if r['id'] == 'mech-revcomp-optimized')
        source = FOLDER / old['path']
        assert run.sha(source) == old['source']['sha256']
        assert run.sha(old['command'][0]) == old['executable_sha256']
        binary = BUILD / 'revcomp-current'
        project = Path(str(binary) + '.project')
        if project.exists():
            __import__('shutil').rmtree(project)
        build_env = env | old['build_environment']
        command = [str(ROOT / 'benchmarks/clbg/nbody/mech-driver/target/release/mech'),
                   'build', str(source), '--backend', 'cranelift', '--offline',
                   '--keep-project', '--no-config', '-o', str(binary)]
        subprocess.run(command, cwd=ROOT, env=build_env, check=True)
        common = {'source': old['source'], 'runtime_environment': old['runtime_environment']}
        rows = [common | {'id': 'revcomp-previous', 'command': old['command'],
                          'executable_sha256': old['executable_sha256'],
                          'native_object_sha256': old['native_object_sha256']},
                common | {'id': 'revcomp-current', 'command': [str(binary)],
                          'executable_sha256': run.sha(binary),
                          'native_object_sha256': run.sha(project / 'mech-sequence.o'),
                          'compiler_executable_sha256': run.sha(command[0]),
                          'generation_command': command}]
        (BUILD / 'prepared.json').write_text(json.dumps(rows, indent=2) + '\n')
        if args.prepare_only:
            return
    else:
        rows = json.loads((BUILD / 'prepared.json').read_text())
    small, full, small_sha, full_sha, _ = previous_run.references()
    for row in rows:
        assert run.sha(row['command'][0]) == row['executable_sha256']
        for input_file, reference in ((small, small_sha), (full, full_sha)):
            _, digest, _, _ = previous_run.execute(row['command'], env | row['runtime_environment'], input_file, 300)
            assert digest == reference, row['id']
        row['small_and_full_output_passed'] = True
        row['samples_seconds'] = []
    rng = random.Random(20261007)
    orders = []
    for sample in range(args.samples):
        order = rows.copy(); rng.shuffle(order); orders.append([r['id'] for r in order])
        for row in order:
            elapsed, digest, _, _ = previous_run.execute(row['command'], env | row['runtime_environment'], full, 300)
            assert digest == full_sha
            row['samples_seconds'].append(elapsed)
            print(f'{sample + 1}/{args.samples} {row["id"]}: {elapsed:.6f}s', flush=True)
    for row in rows:
        row['median_seconds'] = statistics.median(row['samples_seconds'])
    output = HERE / 'results/revcomp-regression.json'
    output.write_text(json.dumps({'programs': rows, 'all_outputs_passed': True,
        'native_object_identical': rows[0]['native_object_sha256'] == rows[1]['native_object_sha256'],
        'samples': args.samples, 'sample_order': orders, 'compiler_snapshot': run.snapshot(output),
        'protocol': 'Sequential fresh processes; full regular-file IO; output hashing excluded.'}, indent=2) + '\n')

if __name__ == '__main__':
    main()
