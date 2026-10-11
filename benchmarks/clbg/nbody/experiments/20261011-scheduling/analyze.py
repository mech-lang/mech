#!/usr/bin/env python3
"""Summarize matched rounds; retain all samples, including losing policies."""
import argparse
import json
import math
import random
import statistics
from pathlib import Path


def comparison(report, candidate, baseline):
    rows = {row['id']: row for row in report['programs']}
    a, b = rows[candidate], rows[baseline]
    ratios = [math.log(x / y) for x, y in zip(a['samples_seconds'], b['samples_seconds'])]
    assert len(ratios) == report['samples']
    rng = random.Random(20261011)
    draws = sorted(math.exp(statistics.mean(rng.choices(ratios, k=len(ratios))))
                   for _ in range(10000))
    return dict(candidate=candidate, baseline=baseline,
                candidate_seconds=a['median_seconds'], baseline_seconds=b['median_seconds'],
                median_ratio=a['median_seconds'] / b['median_seconds'],
                paired_geomean_ratio=math.exp(statistics.mean(ratios)),
                paired_bootstrap_95=[draws[250], draws[9749]])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('report', type=Path)
    parser.add_argument('--baseline', default='pairwise-seed8')
    parser.add_argument('--candidate')
    args = parser.parse_args()
    report = json.loads(args.report.read_text())
    assert report['status'] == 'complete'
    if args.candidate:
        print(json.dumps(comparison(report, args.candidate, args.baseline), indent=2))
        return
    baseline = next(row for row in report['programs'] if row['id'] == args.baseline)
    print('program,seconds,time_x_mech,LOC,LOC_x_mech,instructions,stack_loads,stack_stores')
    for row in sorted(report['programs'], key=lambda x: x['median_seconds']):
        code = row.get('codegen', {})
        loc = row.get('loc', '')
        relative_loc = loc / baseline['loc'] if isinstance(loc, (int, float)) else ''
        print(','.join(map(str, [row['id'], row['median_seconds'],
              row['median_seconds'] / baseline['median_seconds'], loc, relative_loc,
              code.get('static_instructions', ''), code.get('stack_load_instructions', ''),
              code.get('stack_store_instructions', '')])))


if __name__ == '__main__':
    main()
