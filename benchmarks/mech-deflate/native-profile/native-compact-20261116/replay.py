"""Recompute historical peer wins from the published raw paired observations."""
from pathlib import Path
import gzip,json,math,statistics

here=Path(__file__).resolve().parent
results={}
for run in ['baseline/full','baseline/full-reverse','cold-tails/focus','cold-tails/focus-reverse']:
    folder=here/'evidence'/run
    summary=json.loads((folder/'summary.json').read_text())
    wins={'roc':0,'libdeflate':0};rows=[]
    for row in summary:
        blocks=json.loads(gzip.decompress((folder/row['case']/'observations.json.gz').read_bytes()))
        ratios={peer:math.exp(statistics.mean(math.log(b['means']['mech-experiment']/b['means'][peer]) for b in blocks)) for peer in ['roc','libdeflate','mech-control']}
        for peer in wins:
            stored=row['paired_x_'+peer]['mech-experiment']
            assert math.isclose(ratios[peer],stored,rel_tol=1e-12)
            wins[peer]+=ratios[peer]<1
        rows.append({'case':row['case'],'ratios':ratios,'blocks':len(blocks)})
    results[run]={'cases':len(rows),'wins':wins,'rows':rows}
assert all(results[r]['wins']=={'roc':81,'libdeflate':61} for r in ['baseline/full','baseline/full-reverse'])
print(json.dumps(results,indent=2))
