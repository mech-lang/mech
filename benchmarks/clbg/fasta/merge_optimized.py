#!/usr/bin/env python3
"""Combine preserved competitors and newly measured compiler policies."""
import argparse
import copy
import json
from pathlib import Path

import run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', choices=('exact', 'streaming'), default='streaming')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.output is None:
        args.output = run.HERE / 'results' / ('apple-m3-optimized-20261008-25000000.json' if args.stage == 'exact'
                                            else 'apple-m3-streaming-20261008-25000000.json')
    names = (
        'apple-m3-final-25000000.json',
        'apple-m3-exact-compare-policies-25000000.json',
        'apple-m3-exact-compare-matched-25000000.json') if args.stage == 'exact' else (
        'apple-m3-final-25000000.json', 'apple-m3-streaming-final-25000000.json')
    paths = [run.HERE / 'results' / name for name in names]
    reports = [json.loads(p.read_text()) for p in paths]
    rows = {r['id']: copy.deepcopy(r) for r in reports[0]['programs'] if r['language'] != 'mech'}
    for path, report in zip(paths[1:], reports[1:]):
        control = next(r['median_seconds'] for r in report['programs'] if r['id'] == 'fasta-gcc-2-llvm-control')
        for old in report['programs']:
            if old['comparison_stage'] == 'before':
                continue
            row = copy.deepcopy(old)
            if row['comparison_stage'] == 'foreign':
                row['id'] = row['id'].removesuffix('-control')
            row.update(measurement_batch=path.name, comparison_control_seconds=control,
                       comparison_time_ratio=row['median_seconds'] / control)
            rows[row['id']] = row
    result = dict(reports[-1], programs=list(rows.values()), excluded=reports[0]['excluded'],
                  measurement_batches=[dict(path=p.name, sha256=run.sha(p), compiler_snapshot=d['compiler_snapshot']) for p, d in zip(paths, reports)])
    result['protocol'] = dict(result['protocol'], samples='3–5',
                              normalization='Each new batch uses its C #2 LLVM control; preserved competitors retain their original per-batch normalization.')
    def endpoints(cohort):
        summary = run.common.endpoint_summary([dict(r, median_seconds=r['comparison_time_ratio']) for r in cohort])
        summary['time_baseline_ratio_to_control'] = summary.pop('time_baseline_seconds')
        summary['normalization_cohort'] = 'Validated local programs normalized to their own C #2 LLVM control.'
        return summary
    result['endpoints'] = endpoints(result['programs'])
    result['strict_endpoints'] = endpoints([r for r in result['programs'] if r['algorithm']['strict']])
    result['nonheroic_endpoints'] = endpoints([r for r in result['programs'] if not r['upstream_starred']])
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print('combined', len(rows), 'builds;', sum(r['algorithm']['strict'] for r in rows.values()), 'strict')


if __name__ == '__main__':
    main()
