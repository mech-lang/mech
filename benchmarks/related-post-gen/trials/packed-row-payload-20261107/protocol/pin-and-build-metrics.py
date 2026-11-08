from pathlib import Path
import json,hashlib,subprocess
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
c=json.loads((P/'latency-config.json').read_text())
for v in c['implementations']:
 for key,target in [('executable','executable_sha256'),('source','source_sha256'),('receipt','receipt_sha256')]:
  if key in v:
   file=Path(v[key]);digest=hashlib.sha256(file.read_bytes()).hexdigest()
   if target in v:assert v[target]==digest,(v['name'],key)
   v[target]=digest
 v['executable_file_bytes']=Path(v['executable']).stat().st_size
(P/'latency-config.json').write_text(json.dumps(c,indent=2)+'\n')
cmd=['python3',str(R/'build-metrics.py'),'--config',str(P/'latency-config.json'),'--out','/private/tmp/mech-related-packed-row-payload-metrics-20261107-v1','--variant','mech-ir277-packed-row-4']
r=subprocess.run(cmd,capture_output=True,text=True)
(P/'metrics-build.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print('metrics-build',r.returncode,flush=True);assert r.returncode==0,r.stderr+r.stdout
old=json.loads(Path('/private/tmp/mech-related-matrix-length-lowering-cohort-20261107-v1/resources-config.json').read_text())
keep={v['name']+'-instrumented' for v in c['implementations'] if v['kind']=='mech' and 'ir277' not in v['name']}
old['implementations']=[v for v in old['implementations'] if v['kind']!='mech' or v['name'] in keep]
new=json.loads(Path('/private/tmp/mech-related-packed-row-payload-metrics-20261107-v1/metrics-config.json').read_text())
old['implementations'] += [v for v in new['implementations'] if v['kind']=='mech']
assert len(old['implementations'])==28
old['authoritative_latency_comparison']='comparison-native-packed-row-payload-20261107-5000-m3.json'
(P/'resources-base.json').write_text(json.dumps(old,indent=2)+'\n')
(P/'resources-config.json').write_text(json.dumps(old,indent=2)+'\n')
