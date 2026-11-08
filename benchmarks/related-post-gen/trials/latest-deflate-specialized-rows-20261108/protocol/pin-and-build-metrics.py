from pathlib import Path
import hashlib,json,subprocess
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
c=json.loads((P/'latency-config.json').read_text())
for v in c['implementations']:
 for key,target in [('executable','executable_sha256'),('source','source_sha256'),('receipt','receipt_sha256')]:
  if key in v:
   digest=hashlib.sha256(Path(v[key]).read_bytes()).hexdigest()
   if target in v:assert v[target]==digest,(v['name'],key)
   v[target]=digest
 v['executable_file_bytes']=Path(v['executable']).stat().st_size
(P/'latency-config.json').write_text(json.dumps(c,indent=2)+'\n')
cmd=['python3',str(R/'build-metrics.py'),'--config',str(P/'latency-config.json'),'--out','/private/tmp/mech-related-latest-deflate-metrics-20261108-v1','--variant','mech-ir280-latest-deflate-4','--variant','mech-ir280-triple-rows-4','--variant','mech-ir280-quartet-rows-4','--variant','mech-ir280-both-rows-4']
r=subprocess.run(cmd,capture_output=True,text=True);(P/'metrics-build.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print('metrics-build',r.returncode,flush=True);assert r.returncode==0,r.stdout+r.stderr
old=json.loads(Path('/private/tmp/mech-related-recovered-scalar-homes-20261108/resources-config.json').read_text());new=json.loads(Path('/private/tmp/mech-related-latest-deflate-metrics-20261108-v1/metrics-config.json').read_text());old['implementations'] +=[v for v in new['implementations']if v['kind']=='mech'];assert len(old['implementations'])==34;old['authoritative_latency_comparison']='comparison-native-latest-deflate-integration-20261108-5000-m3.json'
(P/'resources-base.json').write_text(json.dumps(old,indent=2)+'\n');(P/'resources-config.json').write_text(json.dumps(old,indent=2)+'\n')
