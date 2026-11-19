from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time,statistics,random,sys
L=Path(__file__).resolve().parent
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP')
source=W/'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
read=lambda p:json.loads(p.read_text())
candidate=sys.argv[1] if len(sys.argv)>1 else 'fused';plan=read(L/'screen-plan.json');plan['orders']=[[candidate if x=='fused' else x for x in order] for order in plan['orders']];required=plan['required_identity_fields'];reference=read(L/'control/run-0.0.json');out=L/('screen' if candidate=='fused' else 'screen-'+candidate);out.mkdir(exist_ok=True)
env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
records=[]
def run(role,name):
 report=out/name;cmd=[str(L/role/'parser'),str(source),str(report)]
 start=time.perf_counter()
 result=subprocess.run(cmd,env=env,capture_output=True,text=True,check=True)
 wall=time.perf_counter()-start
 observation=read(report.with_suffix('.0.json'))
 assert all(observation[k]==reference[k] for k in required),(role,observation)
 record={'actor':role,'command':cmd,'process_wall_seconds':wall,**observation}
 (out/(name+'.receipt.json')).write_text(json.dumps(record,indent=2)+'\n')
 return record
for actor in ['control',candidate]:run(actor,'warmup-'+actor)
for n,order in enumerate(plan['orders']):
 pair={actor:run(actor,f'pair-{n}-{actor}') for actor in order};records.append(pair)
 print(n,{actor:round(r['parse_seconds'],6) for actor,r in pair.items()},flush=True)
rng=random.Random(20261119)
def interval(xs):
 medians=sorted(statistics.median(rng.choices(xs,k=len(xs))) for _ in range(20000))
 return [medians[int(.05*len(medians))],medians[int(.95*len(medians))]]
summary={'plan':plan,'actors':{actor:{'binary_sha256':sha256((L/actor/'parser').read_bytes()).hexdigest()} for actor in ['control',candidate]},'identity':{k:reference[k] for k in required},'records':records,'analysis':{}}
for key in plan['timing_fields']:
 differences=[r[candidate][key]-r['control'][key] for r in records];ratios=[r[candidate][key]/r['control'][key] for r in records]
 summary['analysis'][key]={'median_control':statistics.median(r['control'][key] for r in records),'median_fused':statistics.median(r[candidate][key] for r in records),'paired_median_difference':statistics.median(differences),'paired_median_difference_ci90':interval(differences),'paired_median_ratio':statistics.median(ratios),'paired_median_ratio_ci90':interval(ratios),'candidate_faster_pairs':sum(x<0 for x in differences)}
(out/'results.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary['analysis'],indent=2),flush=True)
