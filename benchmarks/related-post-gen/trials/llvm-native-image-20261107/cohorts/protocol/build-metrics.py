from pathlib import Path
import json,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');M=Path('/private/tmp/mech-related-llvm-native-image-metrics-20261107-v1');x=json.loads((P/'metrics-input-config.json').read_text());names=[v['name']for v in x['implementations']if v['name'].endswith('-apple-m3')or v['name'].startswith('mech-llvm-')];assert len(names)==12
cmd=['python3',str(R/'build-metrics.py'),'--config',str(P/'metrics-input-config.json'),'--out',str(M)]
for name in names:cmd+=['--variant',name]
t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);(P/'metrics-build-run.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(r.stdout,flush=True);assert r.returncode==0,r.stderr
new=json.loads((M/'metrics-config.json').read_text());old=json.loads(Path('/private/tmp/mech-related-ordered-rust-controls-cohort-20261107-v1/resources-config.json').read_text());cpu=[]
for v in new['implementations'][1:]:
 if v['name']=='rust-standard-instrumented':v['name']='rust-apple-m3-instrumented'
 if v['kind']=='upstream':
  assert v['rustflags']=='-C target-cpu=apple-m3'
  assert json.loads(Path(v['receipt']).read_text())['rustflags']=='-C target-cpu=apple-m3'
 cpu.append(v)
assert len(cpu)==12
old['implementations']+=cpu;assert len({v['name']for v in old['implementations']})==len(old['implementations'])==24
old.update(authoritative_latency_comparison=x['authoritative_latency_comparison']);(P/'resources-config-base.json').write_text(json.dumps(old,indent=2)+'\n');print('12 new resource adapters built;24 resource variants configured',flush=True)
