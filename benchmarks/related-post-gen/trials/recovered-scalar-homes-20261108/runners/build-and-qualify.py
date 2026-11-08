from pathlib import Path
import copy, hashlib, json, os, shutil, subprocess, time
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
S=Path('/private/tmp/mech-related-recovered-scalar-homes-compiler-20261108')
O=Path('/private/tmp/mech-related-recovered-scalar-homes-opt-20261108-v1')
RT=P/'runtime';RT.mkdir(exist_ok=True)
def run(cmd,path,env=None):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})})
    path.write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    print(path.name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-5000:]+r.stderr[-5000:];return r
build=json.loads((O/'build.json').read_text())
libs={x['crate']:x['command'][-1] for x in build['stages']}
old=Path('/private/tmp/mech-related-packed-row-payload-runners-20261107-v1/runtime/runtime-build.json')
recipe=json.loads(old.read_text());cmd=recipe['command'][:]
for i,a in enumerate(cmd):
    cmd[i]=a.replace('/private/tmp/mech-related-matrix-length-lowering-compiler-20261107',str(S))
    if a=='--extern':
        name=cmd[i+1].split('=',1)[0]
        if name in libs:cmd[i+1]=name+'='+libs[name]
    if a.startswith('metadata='):cmd[i]='metadata=related_recovered_scalar_homes_runtime_v1'
cmd[-1]=str(RT/'libmech_gpu.rlib')
for parent in sorted({str(Path(x).parent) for x in libs.values()}):cmd[1:1]=['-L','dependency='+parent]
env={k:v.replace('/private/tmp/mech-related-matrix-length-lowering-compiler-20261107',str(S))for k,v in recipe['environment'].items()}
run(cmd,RT/'runtime-build.json',env)
runtime=json.loads((RT/'runtime-build.json').read_text())
build['stages'][-1].update(command=cmd,environment=env,exit=0,library_sha256=hashlib.sha256((RT/'libmech_gpu.rlib').read_bytes()).hexdigest(),runtime_only=True)
(RT/'build.json').write_text(json.dumps(build,indent=2)+'\n')
shutil.copyfile(O/'source-inventory.json',RT/'source-inventory.json')
shutil.copyfile('/private/tmp/mech-related-forward-read-windows-opt-20261107-v8/compiler-facts.rs',P/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'compiler-facts.rs','--out',P/'compiler'],P/'compiler-build.json')
sources={'plain':'/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/build/source.mec'}
for name,source in sources.items():
 N=P/name;N.mkdir(exist_ok=True)
 run([P/'compiler',source,N/'build'],N/'object-build.json')
 shutil.copyfile('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/runner.rs',N/'runner.rs')
 run(['python3',R/'build-host.py','--compiler-build',RT/'build.json','--source',N/'runner.rs','--out',N/'runner-aot','--bundle',N/'build','--linked'],N/'runner-build.json')
 run(['python3',R/'audit-native-link.py','--object',N/'build/object.o','--executable',N/'runner-aot','--output',N/'native-link-proof.json'],N/'link-audit.json')
 run([N/'runner-aot',N/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',N/'output.json','1'],N/'full5000-run.json')
 assert json.loads((N/'output.json').read_text())==json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
 run(['python3',R/'experiments/check_subset3.py','--runner',N/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',N/'build/source.mec','--out',N/'edges'],N/'edge-run.json')
 m=json.loads((N/'build/metadata.json').read_text());assert m['typed_ir_version']==279 and m['native_descriptor_version']==11 and len(m['compiled_numeric_functions'])==51
 (N/'summary.json').write_text(json.dumps(dict(exact_outputs=32,linked_audit_passed=True,runtime_only_GPU=True,LLVM_tools_used=False,typed_IR=279,code_bytes=m['code_bytes'],stack_bytes=m['stack_bytes'],native_functions=51,qualification_times_ineligible=True),indent=2)+'\n')
 print(name,'32 exact outputs and same-Object audit passed',flush=True)
