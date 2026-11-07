import hashlib, importlib.util, json, os, pathlib, re, shutil, subprocess, time, sys

S = pathlib.Path('/private/tmp/mech-related-llvm-object-backend-compiler-20261107')
O = pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-opt-20261107-v5')
Q = pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-qualification-20261107-v5')
R = pathlib.Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
LLVM = pathlib.Path('/opt/homebrew/opt/llvm/bin')
Q.mkdir(exist_ok=True)
SDK=subprocess.check_output(['/usr/bin/xcrun','--show-sdk-path'],text=True).strip()
assert pathlib.Path(SDK).is_dir()
build = json.loads((O/'build.json').read_text())
assert len(build['stages']) == 12 and all(s['exit']==0 for s in build['stages'])
spec=importlib.util.spec_from_file_location('compiler_helpers',R/'build-compiler.py');helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
inventory=json.loads((O/'source-inventory.json').read_text())
assert helper.source_inventory(S)==inventory['files']
sha=lambda p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()

def run(cmd,name,env=None,cwd=None,expected=0):
    cmd=list(map(str,cmd));start=time.monotonic()
    prior=Q/(name+'.json')
    if '--resume' in sys.argv and prior.exists():
        saved=json.loads(prior.read_text())
        if saved['command']==cmd and saved['exit']==0:
            print(name,'reuse verified successful command',flush=True)
            return saved
    result=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})},cwd=cwd)
    receipt=dict(command=cmd,environment=env or {},exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr)
    (Q/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
    print(name,result.returncode,flush=True)
    assert result.returncode==expected,result.stdout[-7000:]+result.stderr[-7000:]
    return receipt

stage=next(s for s in build['stages']if s['crate']=='mech_gpu')
cmd=stage['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(Q/'gpu-tests')
for i,x in enumerate(cmd):
    if x.startswith('metadata='):cmd[i]=x+'_llvm_ssa_tests'
run(cmd,'gpu-tests-build',env=stage['environment'])
run([Q/'gpu-tests','llvm_ssa','--test-threads=1','--nocapture'],'ssa-tests')
if '--ir-only' not in sys.argv:
    run([Q/'gpu-tests','llvm_ssa_native','--ignored','--test-threads=1','--nocapture'],'native-semantics-tests',env={'MECH_LLVM_TEST_CLANG':str(LLVM/'clang'),'SDKROOT':SDK})

source=(R/'compiler.rs').read_text().split('fn run() {',1)[0]
source=source.replace('NativeProgramLimitMode, emit_program_object_with_limits','NativeProgramLimitMode, emit_program_llvm_ir_with_limits')
source+='''fn main() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let a=std::env::args().skip(1).collect::<Vec<_>>();assert_eq!(a.len(),2);
        let text=std::fs::read_to_string(&a[0]).unwrap();let artifact=compile(&text);
        let llvm=emit_program_llvm_ir_with_limits(&artifact,&mech_stdlib::source_catalog(),NativeProgramLimitMode::NoLimits.compilation()).unwrap();
        std::fs::write(&a[1],llvm).unwrap();
    }).unwrap().join().unwrap();
}
'''
(Q/'compiler-llvm-ir.rs').write_text(source)
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',Q/'compiler-llvm-ir.rs','--out',Q/'compiler-llvm-ir'],'ir-exporter-build')
summary={}
for name in ['after-4','reuse-4']:
    p=Q/name;p.mkdir(exist_ok=True)
    source=pathlib.Path('/private/tmp/mech-related-forward-read-windows-runners-20261107-v8')/name/'build/source.mec'
    run([Q/'compiler-llvm-ir',source,p/'native.ll'],name+'-ssa-lowering')
    definitions=re.findall(r'^define .*? @"([^"\n]+)"\(', (p/'native.ll').read_text(),re.M)
    assert definitions and len(definitions)==len(set(definitions))
    (p/'definitions.json').write_text(json.dumps(definitions,indent=2)+'\n')
    run([LLVM/'opt','-passes=default<O3>','-mcpu=apple-m3','-S',p/'native.ll','-o',p/'optimized.ll'],name+'-llvm-opt')
    run([LLVM/'llc','-O=3','-global-isel=0','-enable-machine-outliner=never','-mcpu=apple-m3','-relocation-model=pic','-filetype=obj','-pass-remarks-analysis=prologepilog','-pass-remarks-filter=prolog-epilog','-pass-remarks-output='+str(p/'remarks.yaml'),p/'optimized.ll','-o',p/'native.o'],name+'-llvm-machine')
    run(['python3',S/'hosts/gpu/src/native_artifact/program/llvm_backend/machine_facts.py','--object',p/'native.o','--remarks',p/'remarks.yaml','--definitions',p/'definitions.json','--output',p/'machine-facts.json'],name+'-machine-facts')
    facts=json.loads((p/'machine-facts.json').read_text())
    summary[name]=dict(source_sha256=sha(source),LLVM_IR_sha256=sha(p/'native.ll'),optimized_IR_sha256=sha(p/'optimized.ll'),functions=len(definitions),facts=facts,Mech_program_executed=False,native_descriptor_constructed=False,qualified_native_image=False)

if '--ir-only' in sys.argv:
    print('LLVM IR and machine metadata stages completed; native execution/full GPU stages pending',flush=True)
    raise SystemExit(0)
run([Q/'gpu-tests','--test-threads=1'],'gpu-full',expected=101)
text=json.loads((Q/'gpu-full.json').read_text())['stdout']
failed=[line.split(' ... FAILED')[0].removeprefix('test ') for line in text.splitlines() if line.startswith('test ')and line.endswith(' ... FAILED')]
expected=['batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable','native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map']
assert sorted(failed)==sorted(expected),failed
assert helper.source_inventory(S)==inventory['files']
summary.update(source_inventory_exact=True,full_GPU_summary=[line for line in text.splitlines()if line.startswith('test result:')],inherited_failures_exact=True,LLVM_program_qualified=False,performance_measured=False)
(Q/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items()if k not in ['after-4','reuse-4']}),flush=True)
