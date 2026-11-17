"""Recompute peer wins for both Mech workers from the published paired blocks."""

import gzip
import json
import math
from pathlib import Path
from statistics import geometric_mean

EVIDENCE = Path(__file__).resolve().parent / 'evidence'
PEERS = ('roc', 'libdeflate')
CONTROL = 'mech-control'
CANDIDATE = 'mech-experiment'


def replay(run, expected_cases, expected_wins):
    folder = EVIDENCE / run
    summary = json.loads((folder / 'summary.json').read_text())
    assert len({row['case'] for row in summary}) == len(summary) == expected_cases, run
    wins = {actor: dict.fromkeys(PEERS, 0) for actor in (CONTROL, CANDIDATE)}
    rows = []
    for row in summary:
        case = row['case']
        with gzip.open(folder / case / 'observations.json.gz', 'rt') as stream:
            means = [block['means'] for block in json.load(stream)]
        ratios = {}
        for actor in wins:
            for peer in PEERS:
                ratio = geometric_mean(block[actor] / block[peer] for block in means)
                assert math.isclose(ratio, row['paired_x_' + peer][actor], rel_tol=1e-12), (run, case, actor, peer)
                wins[actor][peer] += ratio < 1
                if actor == CANDIDATE:
                    ratios[peer] = ratio
        ratios[CONTROL] = geometric_mean(block[CANDIDATE] / block[CONTROL] for block in means)
        rows.append({'case': case, 'ratios': ratios, 'blocks': len(means)})
    assert wins[CANDIDATE] == expected_wins, run
    return {'cases': expected_cases, 'wins': wins[CANDIDATE], 'control_wins': wins[CONTROL], 'rows': rows}


def main():
    runs = {
        'baseline/full': (81, {'roc': 81, 'libdeflate': 61}),
        'baseline/full-reverse': (81, {'roc': 81, 'libdeflate': 61}),
        'cold-tails/focus': (15, {'roc': 15, 'libdeflate': 10}),
        'cold-tails/focus-reverse': (15, {'roc': 15, 'libdeflate': 10}),
        'owner-cleanup-focus/runs/focus': (12, {'roc': 12, 'libdeflate': 8}),
        'owner-cleanup-focus/runs/focus-reverse': (12, {'roc': 12, 'libdeflate': 8}),
        'owner-cleanup-full/runs/full': (81, {'roc': 81, 'libdeflate': 61}),
        'owner-cleanup-full/runs/full-reverse': (81, {'roc': 81, 'libdeflate': 61}),
    }
    report = json.loads((EVIDENCE / 'fresh-owner-counts/report.json').read_text())
    for variant in ('counts', 'counts-cleanup'):
        measurements = report['variants'][variant]
        for kind, key, cases in [('focus', 'focused_orders', 17), ('full', 'full_orders', 81)]:
            for suffix, timing in zip(('', '-reverse'), measurements.get(key, [])):
                runs[f'fresh-owner-counts/{variant}/runs/{kind}{suffix}'] = (
                    cases, {'roc': timing['roc_wins'][1], 'libdeflate': timing['libdeflate_wins'][1]},
                )
    print(json.dumps({run: replay(run, *spec) for run, spec in runs.items()}, indent=2))


if __name__ == '__main__':
    main()
