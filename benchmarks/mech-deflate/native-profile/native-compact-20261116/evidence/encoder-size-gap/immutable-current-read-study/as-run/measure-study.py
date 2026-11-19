"""Balanced eight-actor complete source-through-link builds; old compiler, disabled control, all four scope/placement variants, Roc and C."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
from statistics import median
import json,os,subprocess,sys,random
ROOT=Path(__file__).resolve().parent;L=ROOT/'placement';I=ROOT.parent;Q=I.parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');read=lambda p:json.loads(Path(p).read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();save=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
base=read(I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/receipt.json');build=read(L/'compiler-build.json');old=read(I/'sealed-base-execution/compiler-build.json');assert build['exit_code']==old['exit_code']==0;assert sha(build['compiler'])==build['compiler_sha256'] and sha(old['compiler'])==old['compiler_sha256'];assert build['source_inputs']=={p:sha(W/p) for p in build['source_inputs']}
variant=sys.argv[1] if len(sys.argv)>1 else 'immutable-profiled';trial=read(L/variant/'receipt.json');disabled=read(L/'disabled-control/receipt.json');source=base['compile'][1];assert sha(source)=='653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
proof=read(L/variant/'linked-proof.json');assert sha(proof['result'])==proof['sha256'] and proof['peak_stack_bytes']<=5280
script=Q/'current-roadmap/shared-terminal-returns/measure.py';sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='immutable_read_complete_measurement');prior=read(I/'sealed-base-execution/matched-builds/plan.json');fixed=prior['actors']['mech-sealed-base-fusion'];CONTROL='mech-control-reference';DISABLED='mech-current-disabled';ROLE='mech-immutable-current-read';PINNED='mech-immutable-only-pinned';BROAD='mech-broad-rematerialized';BROAD_PINNED='mech-broad-pinned'
def prepare(controls,variants):
 assert controls==['reference'] and variants==['immutable-read'];actors={}
 specs=[(CONTROL,old['compiler'],base,I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/frontend'),(DISABLED,build['compiler'],disabled,L/'disabled-control/frontend'),(ROLE,build['compiler'],trial,L/variant/'frontend')]
 for label,stage,compiler in [(PINNED,'immutable-only',build['compiler']),(BROAD,'repair',None),(BROAD_PINNED,'pinned',None)]:
  path=ROOT/stage/'immutable-profiled';record=read(path/'receipt.json');specs.append((label,compiler or record['compile'][0],record,path/'frontend'))
 for role,compiler,record,front in specs:
  pins={'encoder':record['object_sha256'],'metadata':record['metadata_sha256'],'bytecode':record['bytecode_sha256']};obj=front/'program.o';assert {k:sha(front/n) for k,n in [('encoder','program.o'),('metadata','metadata.json'),('bytecode','program.mecb')]}==pins
  actors[role]=fixed|{'compile':[compiler,source,str(front),'compact'],'compiler_sha256':sha(compiler),'reference_object':str(obj),'flags':record['flags'],'link':[str(obj) if arg==fixed['reference_object'] else arg for arg in fixed['link']],'artifact_sha256':pins,'text_bytes':record['text_bytes']}
 actors.update({peer:prior['actors'][peer] for peer in lab['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256'];assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256'];return actors,timer
lab['acquire'].__globals__.update(BASE='5ccf097dc02ceb3abb17bccb85cd11a3ae144503',construct=prepare);output=L/'matched-builds';lab['acquire'](output,['reference'],['immutable-read']);raw=read(output/'comparison.json');rows=raw['observations'];roles=raw['medians'];section='source_through_link';rng=random.Random(20261119)
def interval(xs):
 samples=sorted(median(rng.choices(xs,k=len(xs))) for _ in range(20000));return [samples[1000],samples[19000]]
changes={}
for a,b in [(ROLE,CONTROL),(ROLE,DISABLED),(DISABLED,CONTROL),(PINNED,DISABLED),(BROAD,DISABLED),(BROAD_PINNED,DISABLED),(ROLE,PINNED)]:
 diff=[row[a]['seconds'][section]-row[b]['seconds'][section] for row in rows];ratios=[row[a]['seconds'][section]/row[b]['seconds'][section] for row in rows]
 changes[a+'_versus_'+b]={'paired_median_seconds':median(diff),'paired_ci90_seconds':interval(diff),'paired_median_ratio':median(ratios),'paired_ci90_ratio':interval(ratios),'faster_pairs':sum(x<0 for x in diff)}
result={key:raw[key] for key in ['plan_sha256','medians','phases','paired_ratios']};result|={'candidate_compiler_sha256':build['compiler_sha256'],'control_compiler_sha256':old['compiler_sha256'],'source_inputs':build['source_inputs'],'trials':len(rows),'changes':changes,'source_cpu_seconds':{r:median(sum(row[r]['phase_cpu_seconds']['source'].values()) for row in rows) for r in roles},'source_rss_bytes':{r:median(row[r]['phase_peak_rss_bytes']['source'] for row in rows) for r in roles},'stripped_executables':{r:raw['warmup'][r]['stripped_executable'] for r in roles},'timing_boundary':'Fresh process, original encoder source through native artifact and final deployment link. Fixed support prebuilt for every actor. Strip separate. Compiler bootstrap excluded.'};save(output/'summary.json',result);print(json.dumps(result,indent=2),flush=True)
