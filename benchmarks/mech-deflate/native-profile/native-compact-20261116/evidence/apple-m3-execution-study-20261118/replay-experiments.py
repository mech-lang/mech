"""Replay all retained optimization timing calls with the unchanged #993 verifier."""
from pathlib import Path
from hashlib import sha256
import json,shutil
R=Path(__file__).resolve().parent;B=Path('/private/tmp/mech-pr993-m3-run')
if B.exists():
 for name in ['qualification_records.py','qualification_timings.py','qualification-policy-v2.json']:
  if not (R/name).exists():shutil.copy2(B/name,R/name)
 if not (R/'canonical-fixtures').exists():shutil.copytree(B/'canonical-fixtures',R/'canonical-fixtures')
from qualification_records import ACTORS,Evidence,Findings,load_fixtures
from qualification_timings import runtime_run

def digest(p):return sha256(p.read_bytes()).hexdigest()
def ref(p):return {'path':str(p.relative_to(R)),'sha256':digest(p)}
policy=json.loads((R/'qualification-policy-v2.json').read_text());findings=Findings();fixtures=load_fixtures(Evidence(R/'canonical-fixtures',findings),policy);reports=[]
for p in sorted(R.glob('*/*/plan.json')):
 folder=p.parent;plan=json.loads(p.read_text());assert digest(p)==(folder/'plan.sha256').read_text().strip()
 assert digest(folder/'compare-library.py')==plan['timer_sha256']
 if not (folder/'result.json').exists():continue
 identities={a:{'worker':plan['artifacts'][a]['worker_sha256'],'metadata':plan['artifacts'][a].get('metadata_sha256')} for a in ACTORS}
 identities.update({k:plan[k] for k in ['environment_sha256','contract_sha256']})
 assert digest(folder/'environment.json')==plan['environment_sha256'];assert digest(folder/'contract.json')==plan['contract_sha256'];orders=[]
 for order in ['forward','reverse']:
  d=folder/order;entry={'id':str(d.relative_to(R)),'kind':'full' if plan['scope']=='full' else 'focus','order':order,'blocks_per_case':plan['blocks']}
  receipt={**entry,'pins':ref(d/'pins.json'),'fixture_inventory':ref(folder/'fixtures.json'),'summary':ref(d/'summary.json'),'complete':ref(d/'complete.json'),'observations':{x.parent.name:ref(x) for x in d.glob('*/observations.json.gz')},'cleanup':{a:ref(d/(a+'.stderr')) for a in ACTORS}}
  result=runtime_run(receipt,fixtures,Evidence(R,findings),identities,policy,entry)
  assert result is not None and set(result['cases'])==set(plan['cases'])
  counts={a:{peer:sum(x[peer][a]<1 for x in result['ratios'].values()) for peer in ['roc','libdeflate']} for a in ACTORS[:2]}
  complete=json.loads((d/'complete.json').read_text());orders.append({'order':order,'cases':len(result['cases']),'timed_calls':complete['timed_calls'],'wins':counts,'receipt':receipt})
 assert not findings.unresolved and not findings.rejected,findings
 assert [{k:v for k,v in x.items() if k!='receipt'} for x in orders]==json.loads((folder/'result.json').read_text())
 report={'status':'passed','plan_sha256':digest(p),'runs':orders};(folder/'replay.json').write_text(json.dumps(report,indent=2)+'\n')
 reports.append({'candidate':plan['candidate'],'scope':plan['scope'],'status':'passed','timed_calls':sum(x['timed_calls'] for x in orders),'plan_sha256':digest(p)})
 print(plan['candidate'],plan['scope'],'raw observations verified',flush=True)
(R/'raw-replay.json').write_text(json.dumps({'status':'passed','experiments':reports,'total_timed_calls_verified':sum(x['timed_calls'] for x in reports)},indent=2)+'\n')
