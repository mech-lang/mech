from pathlib import Path
import copy, hashlib, json, shutil, subprocess, time
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-canonical-schema-sinks-opt-20261107-v1')
SU=Path('/private/tmp/mech-related-ordered-subsets-reused-masks-source-20261106-v1')
read=lambda p:json.loads(p.read_text())
def run(command,path):
    command=list(map(str,command));start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True)
    path.write_text(json.dumps(dict(command=command,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr),indent=2)+'\n')
    assert result.returncode==0,result.stdout+result.stderr
    print(path.name,'passed',flush=True)
run(['python3',U/'build-linked-and-check.py'],U/'qualify-all-originals-run.json')
p=U/'reuse-4';p.mkdir();shutil.copyfile(SU/'source.mec',p/'source.mec');shutil.copyfile(SU/'runner.rs',p/'runner.rs')
run([O/'compiler',p/'source.mec',p/'build'],p/'compile-run.json')
metadata=read(p/'build/metadata.json');assert metadata['typed_ir_version']==238
assert next(x for x in metadata['numeric_tree_candidates']if x['iteration']==3)['selected']
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'source.mec','--out',p/'edges'],p/'edge-run.json')

base=read(SU/'comparison-config.json')
config=copy.deepcopy(base)
config['authoritative_latency_comparison']='comparison-native-boolean-implication-mask-reuse-randomized-20261106-5000-m3.json'
config['implementations']=[copy.deepcopy(v)for v in base['implementations']if v['kind']!='mech']
variants={v['name']:v for v in base['implementations']}
for name,old,current in [('mech-parent-original-4','mech-original-4',None),('mech-parent-reuse-4','mech-reuse-4',None),('mech-current-original-4','mech-original-4','after-4'),('mech-current-reuse-4','mech-reuse-4','reuse-4')]:
    item=copy.deepcopy(variants[old]);item['name']=name
    if current:
        root=U/current
        item.update(category='IR238_'+current,executable=str(root/'runner-aot'),source=str(root/'build/source.mec'),receipt=str(root/'runner-aot.build.json'))
    config['implementations'].append(item)
assert len(config['implementations'])==9
(U/'comparison-config.json').write_text(json.dumps(config,indent=2)+'\n')
run(['python3',R/'build-metrics.py','--config',U/'comparison-config.json','--out',U/'metrics','--variant','mech-current-original-4','--variant','mech-current-reuse-4'],U/'metrics-build-run.json')
resource=read(SU/'resource-config.json')
resource['authoritative_latency_comparison']=config['authoritative_latency_comparison']
resource['implementations']=[v for v in resource['implementations']if v['kind']!='mech']
old_resource=read(SU/'resource-config.json')
for before,after in [('mech-original-4-instrumented','mech-parent-original-4-instrumented'),('mech-reuse-4-instrumented','mech-parent-reuse-4-instrumented')]:
    item=copy.deepcopy(next(v for v in old_resource['implementations']if v['name']==before));item['name']=after;resource['implementations'].append(item)
resource['implementations'].extend(v for v in read(U/'metrics/metrics-config.json')['implementations']if v['name']!='rust')
assert len(resource['implementations'])==10
(U/'resource-config.json').write_text(json.dumps(resource,indent=2)+'\n')
print('Fourteen original loaders and new source passed; nine latency/ten resource entries prepared',flush=True)
