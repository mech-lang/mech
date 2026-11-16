from collections import defaultdict
from math import exp, log
from pathlib import Path
from random import Random
from statistics import mean
import gzip
import json

# A 0.5% elapsed-time difference is the decision resolution, independent of
# the observed same-worker spread. Estimates and intervals remain unrounded.
RESOLUTION = 0.005
RESAMPLES = 8000
BASELINE = 'mech-control'
CANDIDATE = 'mech-experiment'

def strata(blocks):
    parts = defaultdict(list)
    for block in blocks:
        order = block['order']
        before_first = order.index(BASELINE) < order.index(CANDIDATE)
        t = block['means']
        parts[before_first].append(log(t[CANDIDATE] / t[BASELINE]))
    return [parts[True], parts[False]]

def interval(parts, seed):
    rng = Random(seed)
    total = sum(map(len, parts))
    center = exp(sum(map(sum, parts)) / total) - 1
    draws = []
    for _ in range(RESAMPLES):
        draws.append(exp(sum(sum(rng.choices(p, k=len(p))) for p in parts) / total) - 1)
    draws.sort()
    return dict(estimate=center, lower=draws[int(RESAMPLES * .05)],
                upper=draws[int(RESAMPLES * .95)])

def local_cohort(root, label, case):
    with gzip.open(root / label / case / 'observations.json.gz', 'rt') as stream:
        return json.load(stream)

def study(root):
    names = [f.name for f in (root/'full').iterdir() if f.is_dir()]
    results = {}
    for i, case in enumerate(sorted(names)):
        read=lambda label: strata(local_cohort(root, label, case))
        aa, full, repeat = map(read, ('repeatability', 'full', 'full-repeat'))
        r = dict(same_worker=interval(aa,i),full=interval(full,i),
                 reverse=interval(repeat,i),combined=interval(full+repeat,i))
        noise = r['same_worker']
        if noise['lower'] < -RESOLUTION or noise['upper'] > RESOLUTION:
            r['decision'] = 'same-worker noise exceeds resolution'
        elif r['combined']['lower'] > RESOLUTION:
            r['decision'] = 'slowdown above resolution'
        elif r['combined']['upper'] <= RESOLUTION:
            r['decision'] = 'bounded at resolution'
        else:
            r['decision'] = 'inconclusive'
        results[case] = r
    return results

if __name__=='__main__':
    from collections import Counter
    root=Path(__file__).resolve().parent
    results=study(root)
    (root/'cold-uncertainty.json').write_text(json.dumps(dict(resolution=RESOLUTION,resamples=RESAMPLES,cases=results),indent=2)+'\n')
    print(Counter(r['decision'] for r in results.values()))
    for name,r in results.items():
        if name in ('empty','seven-stored-prefix','random-64','prefix-natural-32768') or r['decision']!='bounded at resolution':
            print(name,r['decision'], 'A/A', [round(100*v,3) for v in r['same_worker'].values()],
                  'A/B', [round(100*v,3) for v in r['combined'].values()])
