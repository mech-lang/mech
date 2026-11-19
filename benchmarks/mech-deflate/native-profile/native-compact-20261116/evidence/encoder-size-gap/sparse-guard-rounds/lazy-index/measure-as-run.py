"""Compare full-scan and affected-equation closure using one rebuilt compiler."""
from pathlib import Path
from runpy import run_path
from statistics import median
from hashlib import sha256
import json,os,random,shutil,subprocess,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE/'sparse-guard-rounds'/(sys.argv[1] if len(sys.argv)>1 else 'lazy-index')
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
script=HERE.parent/'current-roadmap/shared-terminal-returns/measure.py'
sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='guard_round_matched_builds')
prior=read(HERE/'matched-checked-phase-codebook-core-private-helpers/plan.json')
fixed=prior['actors']['mech-control-reference']
base=read(ROOT/'source-control/rebuild.json')
assert base['full_integrated_source_identity'] and base['object_text_bytes']==63804
binary=Path(base['phases']['source_to_native']['command'][0]);assert sha(binary)==base['compiler_sha256']
compiler=ROOT/'compiler'/sha(binary)[:16]/'compile_profile';compiler.parent.mkdir(parents=True,exist_ok=True)
if not compiler.exists():shutil.copy2(binary,compiler)
assert sha(compiler)==base['compiler_sha256']
pins={'encoder':base['object_sha256'],'metadata':base['metadata_sha256'],'bytecode':base['bytecode_sha256']}
assert pins==fixed['artifact_sha256']
CONTROL='mech-control-reference';ROLE='mech-sparse-rounds'

def prepare(controls,variants):
 assert controls==['reference'] and variants==['rounds']
 actors={}
 for role,label,enabled in [(CONTROL,'scan','0'),(ROLE,'sparse','1')]:
  trial=ROOT/label;trial.mkdir();output=trial/'frontend'
  command=[str(compiler),base['phases']['source_to_native']['command'][1],str(output),'compact']
  flags=base['native_flags']|{'MECH_GUARD_SPARSE_ROUNDS':enabled}
  with (trial/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
  assert all(sha(output/name)==pins[key] for name,key in [('program.o','encoder'),('metadata.json','metadata'),('program.mecb','bytecode')])
  obj=output/'program.o'
  actors[role]=fixed|{'compile':command,'compiler_sha256':sha(compiler),'reference_object':str(obj),'flags':flags,'link':[str(obj) if arg==fixed['reference_object'] else arg for arg in fixed['link']],'artifact_sha256':pins,'text_bytes':63804}
 assert {k:v for k,v in actors[CONTROL]['flags'].items() if k!='MECH_GUARD_SPARSE_ROUNDS'}=={k:v for k,v in actors[ROLE]['flags'].items() if k!='MECH_GUARD_SPARSE_ROUNDS'}
 actors.update({peer:prior['actors'][peer] for peer in lab['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256']
  assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256']
 return actors,timer

lab['acquire'].__globals__.update(BASE=base['source_commit'],construct=prepare)
output=ROOT/'matched-builds';lab['acquire'](output,['reference'],['rounds'])
raw=read(output/'comparison.json');rows=raw['observations'];section='source_through_link'
deltas=[(r[ROLE]['seconds'][section]-r[CONTROL]['seconds'][section])*1000 for r in rows]
rng=random.Random(9951119);resamples=sorted(median(rng.choices(deltas,k=len(rows))) for _ in range(20000))
result={key:raw[key] for key in ('plan_sha256','medians','phases','paired_ratios')}
result.update(source_commit=base['source_commit'],compiler_sha256=sha(compiler),trials=len(rows),paired_ms_cost=deltas,paired_median_ms_cost=median(deltas),cost_90pct_interval_ms=[resamples[999],resamples[18999]],faster=sum(d<0 for d in deltas),source_cpu_seconds={r:median(sum(row[r]['phase_cpu_seconds']['source'].values()) for row in rows) for r in raw['medians']},source_rss_bytes={r:median(row[r]['phase_peak_rss_bytes']['source'] for row in rows) for r in raw['medians']},stripped_executables={r:raw['warmup'][r]['stripped_executable'] for r in raw['medians']},artifact_sha256=pins,seed=9951119,bootstrap_resamples=20000)
(output/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['stripped_executables','artifact_sha256']},indent=2),flush=True)
