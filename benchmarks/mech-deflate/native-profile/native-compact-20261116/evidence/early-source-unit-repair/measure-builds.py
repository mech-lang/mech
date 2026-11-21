from pathlib import Path
from hashlib import sha256
from runpy import run_path
from statistics import median
import json,sys,random,subprocess
D=Path(__file__).resolve().parent;S=D/'scoped-registers';I=D.parent;Q=I.parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');read=lambda p:json.loads(Path(p).read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();save=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
source_pin=subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip();pins=read(S/'source-identities.json');assert pins=={p:sha(W/p) for p in pins}
prior=read(I/'collector-read-lifetimes/final-integration/matched-builds/plan.json');fixed=prior['actors']['mech-final-source'];new=read(S/'build.json');minimal=read(D/'minimal-integration/build.json');script=Q/'current-roadmap/shared-terminal-returns/measure.py';sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='source_unit_build_measurement');CONTROL='mech-control-reference'
def prepare(controls,variants):
 assert controls==['reference'] and variants==['source-units'];actors={CONTROL:fixed}
 jobs=[('mech-final-minimal','selected-map',minimal['compiler']),('mech-current-experimental','selected-map',new['compiler']),('mech-early-unrepaired','early-no-remap',new['compiler']),('mech-early-remapped','early-remapped',new['compiler']),('mech-early-owner-packets','early-owner-homes-shared',new['compiler']),('mech-early-repaired','early-owner-output-root-preserved',new['compiler'])]
 for role,case,compiler in jobs:
  r=read(S/case/'result.json');encoder=str(S/case/'frontend/program.o');flags=r['flags'].copy();flags.pop('MECH_COMPACT_SOURCE_BLOCK_REPORT',None);link=[encoder if arg==fixed['reference_object'] else arg for arg in fixed['link']];assert link.count(encoder)==1;command=r['command'].copy();command[0]=compiler
  job=fixed|{'compile':command,'compiler_sha256':sha(compiler),'link':link,'reference_object':encoder,'flags':flags,'artifact_sha256':dict(zip(['encoder','metadata','bytecode'],[r['artifacts'][n] for n in ['program.o','metadata.json','program.mecb']])),'text_bytes':r['encoder_text_bytes'],'compiler_source_commit':source_pin,'scope':'Architecture experiment; runtime wins not inherited. See actual worker checks and separate stack results.'};job.pop('linked_proof_sha256',None)
  if case=='selected-map':job['scope']='Current source/compiler, identical selected encoder artifacts.'
  if (S/case/'linked-proof.json').exists():job['linked_proof_sha256']=read(S/case/'linked-proof.json')['sha256']
  actors[role]=job
 actors.update({peer:prior['actors'][peer] for peer in ['roc','libdeflate']})
 for job in actors.values():assert sha(job['compile'][0])==job['compiler_sha256'];assert all(sha(path)==h for path,h in job['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256'];return actors,timer
lab['acquire'].__globals__.update(BASE=source_pin,construct=prepare);O=D/'matched-builds';lab['acquire'](O,['reference'],['source-units']);raw=read(O/'comparison.json');rows=raw['observations'];rng=random.Random(9951119);comparisons={}
for actor,reference in [('mech-final-minimal',CONTROL),('mech-current-experimental','mech-final-minimal'),('mech-early-unrepaired','mech-current-experimental'),('mech-early-remapped','mech-early-unrepaired'),('mech-early-owner-packets','mech-early-remapped'),('mech-early-repaired','mech-early-owner-packets'),('mech-early-repaired','mech-current-experimental')]:
 diff=[r[actor]['seconds']['source_through_link']-r[reference]['seconds']['source_through_link'] for r in rows];samples=sorted(median(rng.choices(diff,k=len(diff))) for _ in range(20000));comparisons[actor+' vs '+reference]={'paired_median_seconds':median(diff),'ci90_seconds':[samples[1000],samples[19000]],'faster_pairs':sum(x<0 for x in diff)}
result={k:raw[k] for k in ['plan_sha256','medians','phases','paired_ratios']};result|={'trials':len(rows),'source_commit':source_pin,'paired_differences':comparisons,'source_cpu_seconds':{role:median(sum(r[role]['phase_cpu_seconds']['source'].values()) for r in rows) for role in raw['medians']},'source_rss_bytes':{role:median(r[role]['phase_peak_rss_bytes']['source'] for r in rows) for role in raw['medians']},'scope':'Balanced fresh-process original source through final deployment link, fixed support prebuilt for all actors. Original source/bytecode unchanged. Every Mech actor emits its pinned complete object and metadata. Experiments have no inherited runtime-win claim; source and runtime qualifications remain separate.'};save(O/'summary.json',result);print(json.dumps(result,indent=2),flush=True)
