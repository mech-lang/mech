"""Both-order single-case diagnostic; not full runtime qualification."""
from pathlib import Path
from hashlib import sha256
import json,os,sys
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
control=LAB/'preserved-registers/preserved/bound';timer=Q/'compare-library.py'
roles=['deferred-rebased-scorer-layout']
plan={'source_commit':'b104d014da5da9b87ef270909507e9bfe35558f8','case':'prefix-natural-4096','blocks_per_case':96,'orders':['forward','reverse'],'scope':'Single-case diagnostic. Does not establish 81/62 preservation or a new 63rd win.','timer_sha256':sha(timer),'roles':{},'load_before':os.getloadavg()}
for role in roles:
 bound=ROOT/role/'bound';r=read(bound/'runtime.json');assert sha(bound/'mech-worker')==r['worker_sha256'];assert sha(bound/'program.o')==r['object_sha256'];plan['roles'][role]={'binding':str(bound/'runtime.json'),'binding_sha256':sha(bound/'runtime.json')}
(ROOT/'probe-rebased-plan.json').write_text(json.dumps(plan,indent=2)+'\n')
source=timer.read_text();old="selected = {case['name'] for case in cases}";assert source.count(old)==1;source=source.replace(old,"selected = {'prefix-natural-4096'}")
old='orders = list(permutations(commands))';assert source.count(old)==1;source=source.replace(old,old+' * 4')
rows=[]
for role in roles:
 for order in ['forward','reverse']:
  label=f'natural4k-scorer-rebased-{role}-{order}'
  sys.argv=[str(timer),label,str(control),str(ROOT/role/'bound')]+(['reverse'] if order=='reverse' else [])
  exec(compile(source,str(timer),'exec'),{'__file__':str(timer),'__name__':'__main__'})
  data=read(Q/label/'summary.json');assert len(data)==1
  rows.append({'role':role,'order':order,'run':str(Q/label),'summary_sha256':sha(Q/label/'summary.json'),'result':data[0]})
(ROOT/'probe-rebased-results.json').write_text(json.dumps({'scope':plan['scope'],'load_after':os.getloadavg(),'runs':rows},indent=2)+'\n')
for row in rows:
 d=row['result'];print(row['role'],row['order'],'Mech/C',d['paired_x_libdeflate']['mech-experiment'],'vs selected',d['paired_x_roc']['mech-experiment']/d['paired_x_roc']['mech-control'],flush=True)
