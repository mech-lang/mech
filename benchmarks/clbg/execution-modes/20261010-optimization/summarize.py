#!/usr/bin/env python3
from pathlib import Path
import hashlib,json,math
HERE=Path(__file__).resolve().parent
current=json.loads((HERE/'timings.json').read_text())
before=json.loads((HERE.with_name('20261010')/'timings.json').read_text())
assert current['status']=='complete'
checks=json.loads((HERE/'conformance.json').read_text());objects=json.loads((HERE/'objects.json').read_text())
assert all(r['correct'] for r in checks) and all(r['identical'] for r in objects['programs'].values())
assert all(all(v['reference_match'] for v in r['sample_validation']) and len(r['samples_seconds'])==3 for r in current['programs'].values())
rows={}
for bench in current['completed_benchmarks']:
 rows[bench]={}
 for mode in ('jit','resident','aot-new'):
  original=min((r for r in before['programs'].values() if r['benchmark']==bench and r['mode']==mode),key=lambda r:r['median_seconds'])
  result=min((r for r in current['programs'].values() if r['benchmark']==bench and r['mode']==mode),key=lambda r:r['median_seconds'])
  rows[bench][mode]=dict(source=result['source_key'],loc=result['loc'],before_seconds=original['median_seconds'],seconds=result['median_seconds'],speedup=original['median_seconds']/result['median_seconds'])
  print(bench,mode,f"{original['median_seconds']:.6f} -> {result['median_seconds']:.6f}s",f"{rows[bench][mode]['speedup']:.3f}x",flush=True)
ratios=[r['ratio'] for k,r in current['changes'].items() if k.endswith(':aot-new')]
summary=dict(timings_sha256=hashlib.sha256((HERE/'timings.json').read_bytes()).hexdigest(),full_samples=sum(len(r['samples_seconds']) for r in current['programs'].values()),semantic_checks=len(checks),native_objects_identical=len(objects['programs']),fastest_by_benchmark=rows,aot_control_geometric_mean_ratio=math.exp(sum(map(math.log,ratios))/len(ratios)),discarded_runs=len(current.get('discarded',[])))
(HERE/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
