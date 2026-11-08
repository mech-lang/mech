from pathlib import Path
import json,subprocess,time,shutil,hashlib
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');N=Path('/private/tmp/mech-related-borrowed-dictionary-values-runners-20261108-v3');S=Path('/private/tmp/mech-related-borrowed-dictionary-values-compiler-20261108');B=P/'histogram-reuse';B.mkdir(exist_ok=True)
def run(c,n):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);(P/(n+'.json')).write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');print(n,x.returncode,flush=True);assert x.returncode==0,x.stderr[-5000:]+x.stdout[-5000:]
files=lambda root:{str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest()for f in root.rglob('*')if f.is_file()}
a,b=files(S/'hosts/gpu'),files(P/'hosts/gpu');assert a.keys()==b.keys();changed=[k for k in a if a[k]!=b[k]];assert changed==['src/native_artifact/codegen/control/mod.rs'],changed;(P/'one-line-source-ablation.json').write_text(json.dumps(dict(changed=changed,default=a[changed[0]],control=b[changed[0]],same_Engine_and_runtime=True),indent=2)+'\n')
run(['python3',R/'build-host.py','--compiler-build',P/'build.json','--source',N/'compiler-facts.rs','--out',P/'compiler'],'compiler-build')
run([P/'compiler',Path('/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/histogram-reuse/build/source.mec'),B/'build'],'object-build')
shutil.copyfile(N/'histogram-reuse/runner.rs',B/'runner.rs')
run(['python3',R/'build-host.py','--compiler-build',N/'runtime/build.json','--source',B/'runner.rs','--out',B/'runner-aot','--bundle',B/'build','--linked'],'runner-build')
run(['python3',R/'audit-native-link.py','--object',B/'build/object.o','--executable',B/'runner-aot','--output',B/'native-link-proof.json'],'link-audit')
run([B/'runner-aot',B/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',B/'output.json','1'],'full5000-run')
assert json.loads((B/'output.json').read_text())==json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
run(['python3',R/'experiments/check_subset3.py','--runner',B/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',B/'build/source.mec','--out',B/'edges'],'edge-run')
m=json.loads((B/'build/metadata.json').read_text());(P/'summary.json').write_text(json.dumps(dict(exact_outputs=32,linked_audit_passed=True,same_Engine_and_runtime=True,only_lowering_emission_disabled=True,typed_IR=m['typed_ir_version'],code_bytes=m['code_bytes'],stack_bytes=m['stack_bytes'],qualification_timers_ineligible=True),indent=2)+'\n');print('emission control32 exact linked outputs',flush=True)
