"""Exact-artifact proof and balanced complete-build comparison for sealed base fusion."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
from statistics import median
import json,os,subprocess,sys,random
L=Path(__file__).resolve().parent;I=L.parent;Q=I.parent;T=I/'natural4k-runtime-target';F=T/'hot-matcher-facts';W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');read=lambda p:json.loads(Path(p).read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();save=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
base=read(F/'guards-profiled/receipt.json');build=read(L/'compiler-build.json');assert build['exit_code']==0 and sha(build['compiler'])==build['compiler_sha256'];assert build['source_files']=={str(p.relative_to(W)):sha(p) for p in (W/'src/syntax/src').rglob('*.rs')}
flags=base['flags'];source=base['compile'][1];assert sha(source)=='653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299';pins={k:base[k+'_sha256'] for k in ['object','metadata','bytecode']}
front=L/'emitted/frontend';front.parent.mkdir();command=[build['compiler'],source,str(front),'compact']
with (front.parent/'compile.log').open('w') as log:subprocess.run(command,cwd=W,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
artifacts={k:sha(front/n) for k,n in [('object','program.o'),('metadata','metadata.json'),('bytecode','program.mecb')]};assert artifacts==pins,(artifacts,pins)
linker=run_path(str(Q/'current-roadmap/checked-root-census/recheck-link.py'),run_name='sealed_parser_link_proof');linked=linker['link_worker'](F/'guards-profiled/bound',front,L/'emitted/bound');expected=read(F/'guards-profiled/bound/runtime.json');assert linked['worker_sha256']==expected['worker_sha256']=='88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4'
record={'source_commit':'2bc4014587fda4f31bdade02abd0cd5144d80e23','compiler_sha256':build['compiler_sha256'],'compile':command,'flags':flags,'source_sha256':sha(source),'artifacts':artifacts,'worker_sha256':linked['worker_sha256'],'prior_full_qualification':'benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/encoder-size-gap/current-matcher-guard-facts/candidate.json','qualification_boundary':'Exact same emitted object, complete metadata, bytecode and fixed-support linked worker. Reuse existing 81Roc/62C full-order runtime evidence; no newly acquired runtime campaign is claimed.'};save(L/'exact-artifacts.json',record);print('All selected encoder artifacts and fixed-support worker reproduce exactly.',flush=True)
script=Q/'current-roadmap/shared-terminal-returns/measure.py';sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='sealed_parser_complete_measurement');prior=read(I/'sparse-guard-rounds/lazy-index/matched-builds/plan.json');fixed=prior['actors']['mech-sparse-rounds'];CONTROL='mech-control-reference';ROLE='mech-sealed-base-fusion'
def prepare(controls,variants):
 assert controls==['reference'] and variants==['sealed-fusion'];actors={}
 for role,compiler,artifact in [(CONTROL,base['compile'][0],F/'guards-profiled/frontend'),(ROLE,build['compiler'],front)]:
  job=[compiler,source,str(artifact),'compact'];obj=artifact/'program.o'
  assert all(sha(artifact/n)==pins[k] for k,n in [('object','program.o'),('metadata','metadata.json'),('bytecode','program.mecb')])
  actors[role]=fixed|{'compile':job,'compiler_sha256':sha(compiler),'reference_object':str(obj),'flags':flags,'link':[str(obj) if arg==fixed['reference_object'] else arg for arg in fixed['link']],'artifact_sha256':{'encoder':pins['object'],'metadata':pins['metadata'],'bytecode':pins['bytecode']},'text_bytes':63692}
 actors.update({peer:prior['actors'][peer] for peer in lab['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256']
  assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256'];return actors,timer
lab['acquire'].__globals__.update(BASE='2bc4014587fda4f31bdade02abd0cd5144d80e23',construct=prepare);output=L/'matched-builds';lab['acquire'](output,['reference'],['sealed-fusion']);raw=read(output/'comparison.json');rows=raw['observations'];roles=raw['medians'];section='source_through_link';rng=random.Random(20261119)
def interval(xs):
 medians=sorted(median(rng.choices(xs,k=len(xs))) for _ in range(20000));return [medians[1000],medians[19000]]
diff=[row[ROLE]['seconds'][section]-row[CONTROL]['seconds'][section] for row in rows];ratios=[row[ROLE]['seconds'][section]/row[CONTROL]['seconds'][section] for row in rows]
result={key:raw[key] for key in ['plan_sha256','medians','phases','paired_ratios']};result|={'candidate_compiler_sha256':build['compiler_sha256'],'control_compiler_sha256':base['compiler_sha256'],'trials':len(rows),'paired_median_seconds':median(diff),'paired_ci90_seconds':interval(diff),'paired_median_ratio':median(ratios),'paired_ci90_ratio':interval(ratios),'candidate_faster_pairs':sum(x<0 for x in diff),'source_cpu_seconds':{r:median(sum(row[r]['phase_cpu_seconds']['source'].values()) for row in rows) for r in roles},'source_rss_bytes':{r:median(row[r]['phase_peak_rss_bytes']['source'] for row in rows) for r in roles},'stripped_executables':{r:raw['warmup'][r]['stripped_executable'] for r in roles},'timing_boundary':'Fresh process, original encoder source through native artifact and final deployment link. Fixed support prebuilt for all actors. Strip reported separately. Compiler bootstrap excluded.'};save(output/'summary.json',result);print(json.dumps(result,indent=2),flush=True)
