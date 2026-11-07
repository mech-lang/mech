from pathlib import Path
import copy, hashlib, json, os, shutil, subprocess, time
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
S=Path('/private/tmp/mech-related-paired-scalar-selects-compiler-20261107')
O=Path('/private/tmp/mech-related-paired-scalar-selects-opt-20261107-v3')
RT=P/'runtime';RT.mkdir(exist_ok=True)
def run(cmd,path,env=None):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})})
    path.write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    print(path.name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-5000:]+r.stderr[-5000:];return r
RT=Path('/private/tmp/mech-related-paired-scalar-selects-runners-20261107-v1/runtime')
assert (RT/'build.json').exists()
shutil.copyfile('/private/tmp/mech-related-llvm-native-image-smoke-20261107-v4/compiler-llvm-image.rs',P/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'compiler-facts.rs','--out',P/'compiler'],P/'compiler-build.json')
source=Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/build/source.mec')
run([P/'compiler',source,P/'build'],P/'object-build.json')
shutil.copyfile('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/runner.rs',P/'runner.rs')
run(['python3',R/'build-host.py','--compiler-build',RT/'build.json','--source',P/'runner.rs','--out',P/'runner-aot','--bundle',P/'build','--linked'],P/'runner-build.json')
run(['python3',R/'audit-native-link.py','--object',P/'build/object.o','--executable',P/'runner-aot','--output',P/'native-link-proof.json'],P/'link-audit.json')
run([P/'runner-aot',P/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',P/'output.json','1'],P/'full5000-run.json')
assert json.loads((P/'output.json').read_text())==json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
run(['python3',R/'experiments/check_subset3.py','--runner',P/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',P/'build/source.mec','--out',P/'edges'],P/'edge-run.json')
metadata=json.loads((P/'build/metadata.json').read_text())
native=metadata['compiled_numeric_functions'];root=metadata['compiled_cfg_cleanup']
assert metadata['typed_ir_version']==268 and metadata['native_descriptor_version']==10
(P/'summary.json').write_text(json.dumps(dict(exact_outputs=32,linked_audit_passed=True,runtime_only_GPU=True,LLVM_tools_used=True,optional_reference_only=True,typed_IR=268,code_bytes=metadata['code_bytes'],stack_bytes=metadata['stack_bytes'],query1282=next(f for f in native if f['header']==1282),paired_selects=metadata['compiled_paired_scalar_selects']),indent=2)+'\n')
