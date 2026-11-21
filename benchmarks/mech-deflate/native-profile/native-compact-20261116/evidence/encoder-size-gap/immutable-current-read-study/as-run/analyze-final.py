"""Bind both complete orders to exact source artifacts and report all case tradeoffs."""
from pathlib import Path
from hashlib import sha256
import json,gzip,math
ROOT=Path(__file__).resolve().parent;L=ROOT/'placement';Q=ROOT.parent.parent;read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
role=L/'immutable-profiled';receipt=read(role/'receipt.json');runtime=read(role/'bound/runtime.json');replay=read(role/'full/full-replay.json');assert replay['decision']=='Retain' and replay['rejected']==replay['unresolved']==[];ratios=[];required=[];wins=[]
for run in replay['runs']:
 assert run['case_count']==81
 for peer,floor in [('roc',81),('libdeflate',62)]:
  assert all(run['peers'][peer][field]['mech-experiment']>=floor for field in ['point_estimate_wins','precision_supported_wins']);assert run['peers'][peer]['lost_cases']==[]
 order=run['order'];folder=Q/run['id'];summary=read(folder/'summary.json');assert len(summary)==81
 peers={p:{'control':[],'candidate':[],'lost':[]} for p in ['roc','libdeflate']}
 for row in summary:
  case=row['case'];data=json.loads(gzip.decompress((folder/case/'observations.json.gz').read_bytes()));blocks=data['blocks'] if isinstance(data,dict) else data;assert len(blocks)==96
  paired={actor:math.exp(sum(math.log(b['means']['mech-experiment']/b['means'][actor]) for b in blocks)/len(blocks)) for actor in ['mech-control','roc','libdeflate']}
  assert abs(paired['roc']-row['paired_x_roc']['mech-experiment'])<1e-10;assert abs(paired['libdeflate']-row['paired_x_libdeflate']['mech-experiment'])<1e-10
  ratios.append({'case':case,'order':order,'candidate_control':paired['mech-control'],'candidate_roc':paired['roc'],'candidate_c':paired['libdeflate'],'median_seconds':row['median_seconds']})
  for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
   for actor,label in [('mech-control','control'),('mech-experiment','candidate')]:
    if row[key][actor]<1:peers[peer][label].append(case)
   if row[key]['mech-control']<1<=row[key]['mech-experiment']:peers[peer]['lost'].append(case)
 assert all(not p['lost'] for p in peers.values());required.append({'order':order,'peers':peers})
 wins.append({'order':order,**{peer:{field:run['peers'][peer][field]['mech-experiment'] for field in ['point_estimate_wins','precision_supported_wins']} for peer in ['roc','libdeflate']}})
write(L/'runtime-case-ratios.json',ratios);write(L/'required-wins.json',required)
by_case={}
for r in ratios:by_case.setdefault(r['case'],[]).append(r)
losses=[]
for case,rows in by_case.items():
 if any(r['candidate_c']>=1 for r in rows):losses.append({'case':case,'worst_ratio':max(r['candidate_c'] for r in rows),'needed_time_reduction':1-1/max(r['candidate_c'] for r in rows),'orders':rows})
losses.sort(key=lambda r:r['worst_ratio']);write(L/'remaining-losses.json',losses)
proof=read(role/'linked-proof.json');recovery=read(role/'recoveries/result.json');assert proof['peak_stack_bytes']==4848 and recovery['cases']==6 and recovery['exact_recoveries']==12
result={'source_integration':read(ROOT/'production-integration.json'),'encoder_bytes':receipt['text_bytes'],'encoder_x_roc':receipt['text_bytes']/57696,'encoder_x_libdeflate':receipt['text_bytes']/37080,'bytes_saved_vs_selected':63692-receipt['text_bytes'],'compiler_sha256':receipt['compiler_sha256'],'source_sha256':'653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299','artifacts':receipt['artifacts'],'worker_sha256':runtime['worker_sha256'],'runtime_decision':replay['decision'],'peak_stack_bytes':proof['peak_stack_bytes'],'source_failure_cases':recovery['cases'],'source_exact_recoveries':recovery['exact_recoveries'],'wins':wins,'relative':replay['candidate_control_timing_observations'],'remaining_c_losses':len(losses),'next_c_target':losses[0],'scope':'Changed actual worker, both full81-case orders,96 paired blocks/case, all individual peer wins and81/62 point+precision floors. Complete builds are recorded separately; code/bytecode are original-source generated.'};write(L/'result.json',result)
print(json.dumps({k:v for k,v in result.items() if k not in ['next_c_target','relative','source_integration']},indent=2));print('relative',replay['candidate_control_timing_observations']);print('nearest C',losses[0]['case'],losses[0]['worst_ratio'])
