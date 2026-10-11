#!/usr/bin/env python3
"""Fresh-process full workloads, common output validation, three randomized rounds."""
from pathlib import Path
import hashlib, importlib.util, json, random, statistics

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]

def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()

def main():
    spec = importlib.util.spec_from_file_location('gap_measure_common', CLBG / 'array-comparison/all-20261009/run.py')
    common = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(common)
    old = json.loads((CLBG / 'execution-modes/20261010/builds.json').read_text())['programs']
    programs = []
    built = json.loads((HERE / 'results/prototype-builds.json').read_text())
    expected = {(benchmark, profile) for benchmark in ['fasta', 'pidigits']
                for profile in ['prototype', 'rebuilt-smallest']} | {('fasta', 'binary-search')}
    assert len(built) == len(expected) and {(row['benchmark'], row['profile']) for row in built} == expected
    for row in built:
        programs.append(row | dict(command=[row['binary'], '{n}'], samples_seconds=[], validations=[]))
    for benchmark, small, fast in [
        ('fasta', 'fasta:mech-fasta-final-compact-v2', 'fasta:mech-fasta-final-expanded-v2'),
        ('pidigits', 'pidigits:mech-pi-final-compact-signed', 'pidigits:mech-pi-final-advance-signed')]:
        for profile, key in [('prior-smallest', small), ('current-fastest', fast)]:
            row = old[key]
            programs.append(dict(benchmark=benchmark, profile=profile, binary=row['binary'],
                binary_sha256=sha(row['binary']), source=row['source'], source_sha256=sha(row['source']),
                command=[row['binary'], '{n}'], runtime_environment=row['runtime_environment'],
                samples_seconds=[], validations=[]))
    report = dict(status='running', sample_count=3, programs=programs, orders=[],
        timing_boundary='Fresh process including runtime setup, computation, complete output and flush; compilation and output validation excluded.',
        workloads={'fasta': 25000000, 'pidigits': 10000})
    def save():
        (HERE / 'results/prototype-timings.json').write_text(json.dumps(report, indent=2) + '\n')
    randomizer = random.Random(2026101104)
    save()
    for sample in range(3):
        order = programs.copy()
        randomizer.shuffle(order)
        report['orders'].append([r['benchmark'] + ':' + r['profile'] for r in order])
        for row in order:
            assert sha(row['binary']) == row['binary_sha256']
            assert sha(row['source']) == row['source_sha256']
            common.compact.wait_for_environment(common.REQUIRED)
            measured = common.execute(row, True, 600)
            assert not measured['competing_compilers'] and not measured['power_changes']
            row['samples_seconds'].append(measured['seconds'])
            row['validations'].append(measured)
            print(sample + 1, row['benchmark'], row['profile'], f"{measured['seconds']:.6f}s", flush=True)
            save()
    for row in programs:
        row['median_seconds'] = statistics.median(row['samples_seconds'])
        anchor = next(r for r in programs if r['benchmark'] == row['benchmark'] and r['profile'] == 'current-fastest')
        row['time_x_mech_aot_fastest'] = row['median_seconds'] / statistics.median(anchor['samples_seconds'])
    report['status'] = 'passed'
    save()
    for row in programs:
        print(row['benchmark'], row['profile'], f"{row['median_seconds']:.6f}s", f"{row['time_x_mech_aot_fastest']:.3f}x Mech AOT", flush=True)

if __name__ == '__main__':
    main()
