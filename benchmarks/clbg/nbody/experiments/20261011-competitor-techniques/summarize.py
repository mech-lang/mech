#!/usr/bin/env python3
"""Freeze the decision from matched, full-size compiler experiments."""
import csv
import hashlib
import json
import math
from pathlib import Path
import random
import statistics

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    final = json.loads((HERE / 'confirmation-50m.json').read_text())
    assert final['status'] == 'complete' and final['steps'] == 50_000_000
    assert final['samples'] == 7 and not final['discarded_samples']
    assert all(check['bit_identical'] for check in final['state_checks'])
    rows = {row['id']: row for row in final['programs']}
    for row in rows.values():
        assert len(row['samples_seconds']) == 7
        assert sha(row['command'][0]) == row['binary_sha256']
        if row['language'] == 'mech':
            assert sha(row['source']) == row['source_sha256']
            assert not row['codegen']['loop_calls']
    comparisons = []
    rng = random.Random(9187)
    for candidate, baseline in [('compact-regions8', 'compact-before'),
                                ('compact-seed', 'compact-before'),
                                ('pairwise-regions16seed', 'pairwise-before')]:
        current, old = rows[candidate], rows[baseline]
        paired = [a / b for a, b in zip(current['samples_seconds'], old['samples_seconds'])]
        bootstrap = sorted(math.exp(statistics.mean(math.log(rng.choice(paired))
                                                   for _ in paired))
                           for _ in range(10_000))
        comparisons.append(dict(candidate=candidate, baseline=baseline,
                                before_seconds=old['median_seconds'],
                                after_seconds=current['median_seconds'],
                                median_ratio=current['median_seconds'] / old['median_seconds'],
                                paired_geomean_ratio=math.exp(statistics.mean(map(math.log, paired))),
                                paired_bootstrap_95=[bootstrap[249], bootstrap[9749]],
                                bootstrap_note='Within-cohort paired-round diagnostic; not a guarantee against host/frequency effects.'))
    reports = {p.name: sha(p) for p in HERE.glob('*-50m.json')}
    compiler_files = ['hosts/gpu/src/fused_f64/' + name
                      for name in ('mod.rs', 'native.rs', 'runner.rs', 'map_runner.rs')]
    compiler_files += ['benchmarks/clbg/nbody/mech-driver/tests/fused.rs',
                      'benchmarks/clbg/mandelbrot/mech-driver/tests/native_map.rs']
    for name in compiler_files:
        assert sha(ROOT / name) == sha(HERE / 'compiler-final' / name)
    independent = json.loads((HERE / 'independent-state-audit.json').read_text())
    objects = json.loads((HERE / 'final-object-verification.json').read_text())
    assert independent['status'] == objects['status'] == 'passed'
    assert all(row['identical_to_measured_object'] for row in objects['programs'])
    selected = dict(fast='pairwise-before', compact='compact-regions8')
    sources = {'nbody-gpp-6-llvm-native': 'nbody-gpp-6.cpp',
               'chapel-3': 'nbody-chapel-3.chpl', 'rust-3': 'nbody-rust-3.rs'}
    summary = dict(status='passed', selected=selected, comparisons=comparisons,
                   measured_programs=len(rows), completed_samples=56,
                   workload=dict(steps=50_000_000, bodies=5, pairs_per_step=10,
                                 velocity_components=3, initial_and_final_energy=True),
                   protocol=final['protocol'], power=final['required_power'],
                   confirmation_report='confirmation-50m.json', reports=reports,
                   compiler_pins={name: sha(ROOT / name) for name in compiler_files},
                   final_compiler_snapshot='compiler-final',
                   final_object_verification_sha256=sha(HERE / 'final-object-verification.json'),
                   independent_state_audit_sha256=sha(HERE / 'independent-state-audit.json'),
                   independent_max_scaled_error=independent['max_scaled_error'],
                   tests=dict(fused=31, native_map=14, failures=0),
                   control_sources={key: dict(source=str(HERE.parents[1] / 'upstream' / name),
                                              source_sha256=sha(HERE.parents[1] / 'upstream' / name))
                                    for key, name in sources.items()},
                   source_changes=False, specialized_host_kernel=False,
                   fast_conclusion='No confirmed gain; retain unchanged 25-LOC fast source and options. Fresh control timing is not a compiler speedup.',
                   compact_conclusion='20-LOC source unchanged. General pressure-aware arithmetic scheduling in 8-instruction compute regions improves the matched median by 2.7%.',
                   summarize_sha256=sha(__file__))
    (HERE / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    reference = rows[selected['fast']]
    with (HERE / 'confirmation-mech.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=['program', 'seconds', 'time_x_mech',
                                              'loc', 'loc_x_mech', 'stdev_seconds'])
        writer.writeheader()
        for row in sorted(rows.values(), key=lambda row: row['median_seconds']):
            writer.writerow(dict(program=row['id'], seconds=row['median_seconds'],
                                 time_x_mech=row['median_seconds'] / reference['median_seconds'],
                                 loc=row['loc'], loc_x_mech=row['loc'] / reference['loc'],
                                 stdev_seconds=row['stdev_seconds']))
    print(json.dumps(dict(status=summary['status'], selected=selected, comparisons=comparisons)))


if __name__ == '__main__':
    main()
