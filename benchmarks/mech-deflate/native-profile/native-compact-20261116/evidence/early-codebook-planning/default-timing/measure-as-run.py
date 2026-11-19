"""Price the normal compiler after observer/planner additions, without diagnostics."""
from pathlib import Path
from runpy import run_path
from statistics import median
from hashlib import sha256
import json,os,random,shutil,subprocess,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE/'early-codebook-gated-build-comparison';ROOT.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
script=HERE.parent/'current-roadmap/shared-terminal-returns/measure.py'
sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='early_default_compiler_measurement')
prior=read(HERE/'sparse-guard-rounds/lazy-index/matched-builds/plan.json')
old=prior['actors']['mech-sparse-rounds'];source=read(HERE/'early-codebook-gated-default-source/rebuild.json')
binary=Path(source['phases']['source_to_native']['command'][0]);assert sha(binary)==source['compiler_sha256']
compiler=ROOT/'compiler'/sha(binary)[:16]/'compile_profile';compiler.parent.mkdir(parents=True,exist_ok=True)
if not compiler.exists():shutil.copy2(binary,compiler)
assert sha(compiler)==source['compiler_sha256']
CONTROL='mech-control-reference';ROLE='mech-current-default'

def prepare(controls,variants):
 assert controls==['reference'] and variants==['current']
 actors={}
 for role,label,executable,flags in [(CONTROL,'prior',old['compile'][0],old['flags']),(ROLE,'current',str(compiler),old['flags'])]:
  trial=ROOT/label;trial.mkdir();output=trial/'frontend'
  command=[executable,source['phases']['source_to_native']['command'][1],str(output),'compact']
  assert all('PREVIEW' not in k and 'CENSUS' not in k and 'DUMP' not in k for k in flags)
  with (trial/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
  assert all(sha(output/name)==old['artifact_sha256'][key] for name,key in [('program.o','encoder'),('metadata.json','metadata'),('program.mecb','bytecode')])
  obj=output/'program.o'
  actors[role]=old|{'compile':command,'compiler_sha256':sha(executable),'reference_object':str(obj),'flags':flags,'link':[str(obj) if arg==old['reference_object'] else arg for arg in old['link']]}
 actors.update({peer:prior['actors'][peer] for peer in lab['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256']
  assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256']
 return actors,timer
lab['acquire'].__globals__.update(BASE=source['source_commit'],construct=prepare)
output=ROOT/'matched';lab['acquire'](output,['reference'],['current'])
raw=read(output/'comparison.json');rows=raw['observations'];deltas=[(r[ROLE]['seconds']['source_through_link']-r[CONTROL]['seconds']['source_through_link'])*1000 for r in rows]
rng=random.Random(9951120);boot=sorted(median(rng.choices(deltas,k=8)) for _ in range(20000))
summary={k:raw[k] for k in ['plan_sha256','medians','phases','paired_ratios']}
summary.update(paired_ms_cost=deltas,paired_median_ms_cost=median(deltas),cost_90pct_interval_ms=[boot[999],boot[18999]],faster=sum(d<0 for d in deltas),trials=8,seed=9951120,bootstrap_resamples=20000,default_compiler_sha256=sha(compiler),prior_compiler_sha256=old['compiler_sha256'],source_cpu_seconds={role:median(sum(r[role]['phase_cpu_seconds']['source'].values()) for r in rows) for role in raw['medians']},source_rss_bytes={role:median(r[role]['phase_peak_rss_bytes']['source'] for r in rows) for role in raw['medians']})
(output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ['medians','phases','paired_ratios']},indent=2))
