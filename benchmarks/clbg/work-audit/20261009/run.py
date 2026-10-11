#!/usr/bin/env python3
"""Build fidelity corrections, compare unchanged native objects, and time separately."""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
COMPILER = CLBG / 'nbody/mech-driver/target/release/mech'
BASELINE = CLBG / 'function-application-audit/20261009T171437Z/application-v3/builds.json'
POWER = dict(source='AC Power', low_power_mode=0)


def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def save(path, value):
    temporary = path.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def load_guard():
    spec = importlib.util.spec_from_file_location('work_audit_guard', CLBG / 'compact/run.py')
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    return guard


def build(args):
    baseline = json.loads(BASELINE.read_text())
    destination = HERE / 'builds.json'
    report = json.loads(destination.read_text()) if destination.exists() else dict(
        created_at=datetime.now(timezone.utc).isoformat(), compiler=str(COMPILER),
        compiler_sha256=sha(COMPILER), baseline=str(BASELINE), baseline_sha256=sha(BASELINE),
        programs={}, unchanged_native_objects={})
    assert sha(COMPILER) == report['compiler_sha256'], 'Use a new audit directory for a new compiler.'
    jobs = []
    for row in baseline['programs']:
        if row['benchmark'] == 'nbody':
            continue  # This compiler change does not touch the reactive numeric backend.
        before = row['builds']['after']
        jobs.append((row, before, 'unchanged', Path(before['source'])))
        if row['benchmark'] == 'fasta':
            jobs.append((row, before, 'floating', Path(before['source'])))
        elif row['benchmark'] == 'spectralnorm':
            jobs.append((row, before, 'four-functions', HERE / 'sources/spectralnorm-four-functions.mec'))
    for row, before, variant, source in jobs:
        key = row['key'] + ':' + variant
        if key in report['programs']:
            previous = report['programs'][key]
            assert sha(source) == previous['source_sha256']
            assert sha(previous['binary']) == previous['binary_sha256']
            continue
        assert sha(before['source']) == before['source_sha256']
        binary = HERE / 'builds' / (row['key'].replace(':', '-') + '-' + variant)
        command = list(before['commands'][0])
        command[0], command[2] = str(COMPILER), str(source)
        command[command.index('-o') + 1] = str(binary)
        environment = dict(os.environ)
        environment.update(before['build_environment'])
        environment.update(C_INCLUDE_PATH='/opt/homebrew/include', LIBRARY_PATH='/opt/homebrew/lib')
        if variant == 'floating':
            environment['MECH_CONTROL_EXACT_FLOAT_COMPARE'] = '0'
        with binary.with_suffix('.build.log').open('w') as handle:
            print('BUILD', key, flush=True)
            subprocess.run(command, cwd=ROOT, env=environment, stdout=handle,
                           stderr=subprocess.STDOUT, check=True)
        assert sha(COMPILER) == report['compiler_sha256']
        project = Path(str(binary) + '.project')
        artifacts = {str(path.relative_to(project)): sha(path) for path in project.rglob('*')
                     if path.is_file() and path.suffix in ('.o', '.rs', '.mec', '.json')}
        result = dict(key=key, benchmark=row['benchmark'], variant=variant,
                      source=str(source), source_sha256=sha(source),
                      loc=sum(bool(line.strip()) and not line.lstrip().startswith('--')
                              for line in source.read_text().splitlines()),
                      binary=str(binary), binary_sha256=sha(binary), command=[str(binary), '{n}'],
                      build_command=command,
                      build_environment={k: v for k, v in environment.items() if k.startswith('MECH_')
                                         or k in ('RUSTFLAGS', 'CARGO_TARGET_DIR', 'C_INCLUDE_PATH', 'LIBRARY_PATH')},
                      artifacts=artifacts, baseline_binary=before['binary'])
        if variant == 'unchanged':
            objects = {name: digest for name, digest in before['artifacts'].items() if name.endswith('.o')}
            matches = {name: artifacts.get(name) == digest for name, digest in objects.items()}
            report['unchanged_native_objects'][row['key']] = matches
            if not objects or not all(matches.values()):
                save(destination, report)
                raise RuntimeError('Unchanged source emitted a different native object: ' + key)
        report['programs'][key] = result
        save(destination, report)
    report['status'] = 'passed'
    save(destination, report)
    print('UNCHANGED OBJECTS', len(report['unchanged_native_objects']), 'matched', flush=True)


def measure(args):
    builds = json.loads((HERE / 'builds.json').read_text())
    assert builds['status'] == 'passed'
    guard = load_guard()
    destination = HERE / 'timings.json'
    report = json.loads(destination.read_text()) if destination.exists() else dict(
        created_at=datetime.now(timezone.utc).isoformat(), status='running',
        builds_sha256=sha(HERE / 'builds.json'), power_profile=POWER,
        sample_count=args.samples, orders={}, programs={},
        protocol='Fresh sequential processes; full standard workload; complete output hashes; randomized rounds; '
                 'median and sample standard deviation; no compiler load; AC power with Low Power Mode off. '
                 'Builds, output hashing, and validation comparisons excluded from elapsed time.')
    assert report['builds_sha256'] == sha(HERE / 'builds.json')
    assert report['sample_count'] == args.samples
    for benchmark in ('spectralnorm', 'fasta'):
        rows = []
        for key, built in builds['programs'].items():
            if built['benchmark'] != benchmark:
                continue
            row = report['programs'].setdefault(key, dict(built, samples_seconds=[], sample_validation=[]))
            rows.append(row)
            for full, field in ((False, 'small_validation'), (True, 'full_validation')):
                if field not in row:
                    guard.wait_for_environment(POWER)
                    result = guard.execute(row, full, args.timeout)
                    row[field] = result['validation']
                    print('VALIDATED', key, field, flush=True)
                    save(destination, report)
        rng = random.Random(20261009 + sum(map(ord, benchmark)))
        for index in range(args.samples):
            order = list(rows)
            rng.shuffle(order)
            report['orders'].setdefault(benchmark, [])
            if len(report['orders'][benchmark]) <= index:
                report['orders'][benchmark].append([r['key'] for r in order])
            for row in order:
                if len(row['samples_seconds']) > index:
                    continue
                while True:
                    guard.wait_for_environment(POWER)
                    assert sha(row['source']) == row['source_sha256']
                    assert sha(row['binary']) == row['binary_sha256']
                    result = guard.execute(row, True, args.timeout, watch_compilers=True, expected_power=POWER)
                    if result['competing_compilers'] or result['power_changes']:
                        report.setdefault('discarded', []).append(dict(key=row['key'], result=result))
                        save(destination, report)
                        continue
                    row['samples_seconds'].append(result['seconds'])
                    row['sample_validation'].append(result['validation'])
                    print('SAMPLE', index + 1, row['key'], result['seconds'], flush=True)
                    save(destination, report)
                    break
        for row in rows:
            row['median_seconds'] = statistics.median(row['samples_seconds'])
            row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
        save(destination, report)
    report['status'] = 'passed'
    save(destination, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=('build', 'measure'))
    parser.add_argument('--samples', type=int, default=5)
    parser.add_argument('--timeout', type=int, default=600)
    args = parser.parse_args()
    assert args.samples >= 3
    (build if args.phase == 'build' else measure)(args)


if __name__ == '__main__':
    main()
