from pathlib import Path
import copy,json,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
B=Path('/private/tmp/mech-related-native-frozen-type-descriptors-runners-20261106-v2')
read=lambda p:json.loads(p.read_text())
c=['python3',str(R/'build-metrics.py'),'--config',str(U/'comparison-config.json'),'--out',str(U/'metrics')]
for name in ['mech-after-4','mech-after-8','mech-u8-ids-u32-4','mech-u8-ids-u32-8']:c.extend(['--variant',name])
t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True)
(U/'metrics-build.json').write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
assert r.returncode==0,r.stdout+r.stderr
config=read(B/'resource-config.json');rows=[]
for row in config['implementations']:
    row=copy.deepcopy(row);name=row['name']
    if name.startswith('mech-') and not name.startswith(('mech-before-','mech-after-','mech-u8-ids-u32-')):continue
    if name.startswith('mech-after-'):row['name']=name.replace('mech-after-','mech-parent-')
    if name.startswith('mech-u8-ids-u32-'):row['name']=name.replace('mech-u8-ids-u32-','mech-compact-before-')
    rows.append(row)
rows.extend(x for x in read(U/'metrics/metrics-config.json')['implementations']if x['kind']=='mech')
assert len(rows)==len({x['name']for x in rows})==51
config['implementations']=rows
config['authoritative_latency_comparison']='comparison-native-fixed-frame-arenas-20261106-5000-m3.json'
(U/'resource-config.json').write_text(json.dumps(config,indent=2)+'\n')
print('Resource config ready with51variants',flush=True)
