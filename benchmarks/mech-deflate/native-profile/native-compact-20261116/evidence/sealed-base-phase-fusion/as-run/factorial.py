from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time,statistics,random
L=Path(__file__).resolve().parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');source=W/'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec';read=lambda p:json.loads(p.read_text());roles=['control','fused','bounds-only','fused-bounds'];orders=[[roles[i] for i in order] for order in [[0,1,3,2],[1,2,0,3],[2,3,1,0],[3,0,2,1]]*2];required=read(L/'screen-plan.json')['required_identity_fields'];reference=read(L/'control/run-0.0.json');out=L/'factorial';out.mkdir(exist_ok=True);env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
plan={'scope':'Isolate deterministic chain fusion from the allowance-before-pop repair. Four fresh uninstrumented processes per round; all identity checks preserved. Screening only, not complete build evidence.','orders':orders,'identity_fields':required,'warmups_per_actor':1,'binary_sha256':{r:sha256((L/r/'parser').read_bytes()).hexdigest() for r in roles}}
(out/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
def run(role,name):
 report=out/name;args=[str(L/role/'parser'),str(source),str(report)];start=time.perf_counter();subprocess.run(args,env=env,capture_output=True,check=True);elapsed=time.perf_counter()-start;data=read(report.with_suffix('.0.json'));assert all(data[k]==reference[k] for k in required)
 return {'command':args,'process_wall_seconds':elapsed,**data}
for r in roles:run(r,'warmup-'+r)
records=[]
for i,order in enumerate(orders):
 row={r:run(r,f'pair-{i}-{r}') for r in order};records.append(row);print(i,{r:round(v['parse_seconds'],6) for r,v in row.items()},flush=True)
rng=random.Random(20261119)
def interval(xs):
 samples=sorted(statistics.median(rng.choices(xs,k=len(xs))) for _ in range(20000));return [samples[1000],samples[19000]]
comparisons={}
for candidate,control in [('fused','control'),('bounds-only','control'),('fused-bounds','control'),('fused-bounds','bounds-only')]:
 differences=[row[candidate]['parse_seconds']-row[control]['parse_seconds'] for row in records];ratios=[row[candidate]['parse_seconds']/row[control]['parse_seconds'] for row in records]
 comparisons[candidate+'_vs_'+control]={'paired_median_seconds':statistics.median(differences),'ci90_seconds':interval(differences),'paired_median_ratio':statistics.median(ratios),'ci90_ratio':interval(ratios),'faster_pairs':sum(x<0 for x in differences)}
summary={'plan':plan,'observations':records,'medians':{r:statistics.median(row[r]['parse_seconds'] for row in records) for r in roles},'comparisons':comparisons};(out/'results.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({'medians':summary['medians'],'comparisons':comparisons},indent=2),flush=True)
