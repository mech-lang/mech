"""Balanced fresh-process complete builds: selected, same-policy unsplit, early unit, Roc and C."""
from pathlib import Path
from runpy import run_path
from statistics import median
from hashlib import sha256
import json,os,random,sys
ROOT=Path(__file__).resolve().parent
LAB=ROOT.parent
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
script=LAB.parent/'current-roadmap/shared-terminal-returns/measure.py'
sys.path.insert(0,str(script.parent))
h=run_path(str(script),run_name='early_unit_build_measurement')
prior=read(LAB/'early-codebook-gated-build-comparison/matched/plan.json')
reference=prior['actors']['mech-current-default']
results=read(ROOT/'normal/result.json')
CONTROL='mech-control-reference'
ROLES={'mech-matched-unsplit':'matched-unsplit','mech-early-codebook':'early-codebook'}
def construct(controls,variants):
 assert controls==['reference'] and variants==['early']
 actors={CONTROL:reference}
 for actor,label in ROLES.items():
  r=results[label];front=ROOT/'normal'/label/'frontend';obj=front/'program.o'
  assert sha(r['command'][0])==r['compiler_sha256']
  assert all(sha(front/name)==pin for name,pin in r['artifacts'].items())
  actors[actor]=reference|{'compile':r['command'],'compiler_sha256':r['compiler_sha256'],'reference_object':str(obj),'flags':r['flags'],
   'link':[str(obj) if arg==reference['reference_object'] else arg for arg in reference['link']],
   'artifact_sha256':{key:r['artifacts'][name] for key,name in [('encoder','program.o'),('metadata','metadata.json'),('bytecode','program.mecb')]}}
 actors.update({peer:prior['actors'][peer] for peer in h['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256']
  assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=h['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256']
 return actors,timer
h['acquire'].__globals__.update(BASE=results['early-codebook']['source_base'],construct=construct)
out=ROOT/'normal/matched-builds';h['acquire'](out,['reference'],['early'])
raw=read(out/'comparison.json');rows=raw['observations'];rng=random.Random(9951121)
summary={k:raw[k] for k in ['plan_sha256','medians','phases','paired_ratios']}
comparisons={}
for control in [CONTROL,'mech-matched-unsplit']:
 deltas=[(r['mech-early-codebook']['seconds']['source_through_link']-r[control]['seconds']['source_through_link'])*1000 for r in rows]
 boot=sorted(median(rng.choices(deltas,k=len(deltas))) for _ in range(20000))
 comparisons[control]={'paired_ms_cost':deltas,'paired_median_ms_cost':median(deltas),'cost_90pct_interval_ms':[boot[999],boot[18999]],'faster':sum(d<0 for d in deltas),'trials':len(deltas)}
summary.update(comparisons=comparisons,seed=9951121,bootstrap_resamples=20000,source_cpu_seconds={role:median(sum(r[role]['phase_cpu_seconds']['source'].values()) for r in rows) for role in raw['medians']},source_rss_bytes={role:median(r[role]['phase_peak_rss_bytes']['source'] for r in rows) for role in raw['medians']})
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
