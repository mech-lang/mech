#!/usr/bin/env python3
"""Validate ordinary runtime policies for fragile published-fastest sources."""
import argparse
import importlib.util
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('scripting_measurement', HERE / 'run.py')
run = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run)


def prepare(args):
    primary = json.loads((HERE / 'results/apple-m3-ac-normal-20261009.json').read_text())
    assert primary['status'] == 'complete', 'Finish the sequential primary sweep first'
    pins = json.loads(run.MANIFEST.read_text())
    by_id = {p['id']: p for p in pins['programs']}
    cpu4 = HERE / 'cpuinfo-4.txt'
    cpu4.write_text(''.join(f'processor\t: {i}\n\n' for i in range(4)))
    report = {key: primary[key] for key in ('hardware', 'toolchains', 'inputs', 'protocol')}
    report.update(schema_version=1, created_at=datetime.now(timezone.utc).isoformat(),
                  status='prepared', programs=[], controls=[], excluded=[],
                  selection_audit=[], completed_benchmarks=[], discarded_samples=[],
                  primary_report_sha256=run.common.sha(HERE / 'results/apple-m3-ac-normal-20261009.json'))
    report['inputs'] = dict(report['inputs'], portability_assets={
        **report['inputs']['portability_assets'], 'cpuinfo-4.txt': run.common.sha(cpu4)})
    report['protocol'] = dict(report['protocol'],
        selection='Runtime-policy checks for published-fastest Ruby fannkuch-redux and PHP binary-trees entries that failed the primary preflight, plus the next listed parallel Python Mandelbrot form after the 64-worker source showed high output-assembly cost on this host. Complete listed sources remain unchanged.',
        policies='Ruby exposes four workers through the general CPU-probe preload; this avoids fractional partition rounding at n=7 under the source’s weighted partitioner. PHP disables JIT to check child-process correctness. These are general runtime settings, not benchmark kernels.')
    specs = [('fannkuchredux-ruby-2', 'ruby', 'cpu4'),
             ('binarytrees-php-7', 'php', 'nojit'),
             ('binarytrees-php-4', 'php', 'nojit'),
             ('binarytrees-php-6', 'php', 'nojit'),
             ('mandelbrot-python3-5', 'python', 'fork')]
    kernel = {name: int(subprocess.check_output(['sysctl', '-n', 'kern.sysv.' + name], text=True))
              for name in ('shmmax', 'shmall', 'shmmni', 'shmseg')}
    report['system_v_shared_memory_limits'] = kernel
    if kernel['shmmax'] >= 32000000:
        specs.append(('mandelbrot-php-3', 'php', 'shared-memory'))
    for identifier, language, policy in specs:
        row = run.candidate(by_id[identifier], language)
        row['id'] += '-' + policy
        row['runtime_policy'] = policy
        if policy == 'cpu4':
            row['runtime_environment']['CLBG_CPUINFO_FILE'] = str(cpu4)
        elif policy == 'nojit':
            row['command'][1:1] = ['-d', 'opcache.jit=off']
            # The later original tracing flag would override an earlier setting.
            row['command'] = [s.replace('opcache.jit=tracing', 'opcache.jit=off') for s in row['command']]
        run.common.wait_for_environment(run.POWER)
        try:
            validations = [run.common.execute(row, False, 60) for _ in range(3)]
            row['correctness_small'] = [v['validation'] for v in validations]
            report['programs'].append(row)
            print('POLICY SMALL PASS', row['id'], flush=True)
        except Exception as error:
            report['excluded'].append(dict(id=row['id'], benchmark=row['benchmark'],
                language=language, stage='runtime-policy small validation', reason=str(error)))
            print('POLICY EXCLUDED', row['id'], str(error), flush=True)
    for benchmark in dict.fromkeys(row['benchmark'] for row in report['programs']):
        control = next(p for p in primary['controls'] if p['benchmark'] == benchmark)
        control = dict(control, samples_seconds=[], sample_validation=[])
        report['controls'].append(control)
    run.save(args.output, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare', action='store_true')
    parser.add_argument('--output', type=Path, default=HERE / 'results/runtime-policies.json')
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--timeout', type=int, default=1800)
    parser.add_argument('--seed', type=int, default=20261010)
    parser.add_argument('--benchmarks', default='auto')
    args = parser.parse_args()
    if args.samples < 3:
        parser.error('At least three samples required')
    if args.prepare:
        assert not args.output.exists(), 'Resume without --prepare'
        prepare(args)
    else:
        if args.benchmarks == 'auto':
            current = json.loads(args.output.read_text())
            args.benchmarks = ','.join(dict.fromkeys(row['benchmark'] for row in current['controls']))
        run.measure(args)
        report = json.loads(args.output.read_text())
        report['status'] = 'complete' if set(report['completed_benchmarks']) == {row['benchmark'] for row in report['controls']} else 'partial'
        run.save(args.output, report)


if __name__ == '__main__':
    main()
