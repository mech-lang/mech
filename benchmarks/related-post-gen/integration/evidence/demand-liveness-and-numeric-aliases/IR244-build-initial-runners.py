from pathlib import Path
import hashlib,json,shutil,subprocess,time
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-demand-liveness-integration-opt-20261107-v2')
B=Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text())
def run(c,p):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stdout+x.stderr
shutil.copyfile('/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1/compiler-facts.rs',O/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',O/'compiler-facts.rs','--out',O/'compiler'],O/'compiler-build-run.json')
proof={}
for name in ['after-4','reuse-4']:
 p=U/name;p.mkdir(exist_ok=True)
 run([O/'compiler',B/name/'build/source.mec',p/'build'],p/'compile-run.json')
 files={f.name:sha(f)for f in (p/'build').iterdir()if f.is_file()};assert len(files)==13
 for file in ['source.mec','artifact.mecb','catalog.rs','catalog-profile.json','map-facts.txt','frame-summary.json']:
  assert files[file]==sha(B/name/'build'/file),(name,file)
 old=read(B/name/'build/metadata.json');new=read(p/'build/metadata.json')
 assert (new['typed_ir_version'],new['abi_version'],new['work_policy_version'],new['native_descriptor_version'])==(244,41,115,9)
 shutil.copyfile(B/name/'runner.rs',p/'runner.rs')
 run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
 run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
 oldf={x['header']:x for x in old['compiled_numeric_functions']};newf={x['header']:x for x in new['compiled_numeric_functions']};assert oldf.keys()==newf.keys()
 facts={k:sum(x['cleanup'].get(k,0)for x in newf.values())for k in ['dead_parameters_removed','dead_instructions_removed','dead_loads_removed']}
 deltas={str(h):dict(before_code=oldf[h]['code_bytes'],after_code=n['code_bytes'],before_stack=oldf[h]['stack_bytes'],after_stack=n['stack_bytes'],cleanup=n['cleanup'])for h,n in newf.items()if oldf[h]['code_bytes']!=n['code_bytes'] or oldf[h]['stack_bytes']!=n['stack_bytes']}
 proof[name]=dict(source_artifact_catalog_frames_exact=True,fixtures=31,before_code_bytes=old['code_bytes'],after_code_bytes=new['code_bytes'],before_stack_bytes=old['stack_bytes'],after_stack_bytes=new['stack_bytes'],native_link_audit=read(p/'native-link-audit.json')['status'],dead_native_values=facts,changed_numeric_functions=deltas,files=files)
 (U/'native-code-comparison.json').write_text(json.dumps(proof,indent=2)+'\n')
 print(name,'audited and31 fixtures exact',old['code_bytes'],new['code_bytes'],facts,flush=True)
