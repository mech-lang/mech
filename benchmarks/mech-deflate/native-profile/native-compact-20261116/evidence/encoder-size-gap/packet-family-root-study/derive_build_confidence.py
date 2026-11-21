"""Recompute paired complete-build bootstrap intervals from saved order pairs."""
from pathlib import Path
import json,random,statistics

def derive(folder):
    rows=json.loads((Path(folder)/'comparison.json').read_text())['observations'];rng=random.Random(184920);result={}
    for actor in ['mech-root-forward','mech-packet-root-combined']:
        pairs=[(v[actor]['seconds']['source_through_link'],v['mech-control-reference']['seconds']['source_through_link']) for v in rows];ratios=[];deltas=[]
        for _ in range(8000):
            sample=[rng.choice(pairs) for _ in pairs];ratios.append(statistics.median(a/b for a,b in sample));deltas.append(statistics.median(b-a for a,b in sample))
        ratios.sort();deltas.sort();result[actor]={'paired_median_ratio':statistics.median(a/b for a,b in pairs),'ratio_percentile90':[ratios[399],ratios[7599]],'paired_median_saved_seconds':statistics.median(b-a for a,b in pairs),'saved_seconds_percentile90':[deltas[399],deltas[7599]],'bootstrap_resamples':8000,'trials':len(pairs),'scope':'Resample complete balanced order pairs. Descriptive90percent percentile interval; not an extra selection gate and not a compiler bootstrap timer.'}
    return result

if __name__=='__main__':
    import sys
    print(json.dumps(derive(Path(sys.argv[1])),indent=2))
