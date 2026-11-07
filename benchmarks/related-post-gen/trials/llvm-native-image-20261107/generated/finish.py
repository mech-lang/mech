from pathlib import Path
import copy,hashlib,json,os,subprocess,time
P=Path(__file__).parent;A=P/'applications';R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');RT=Path('/private/tmp/mech-related-llvm-native-image-runtime-20261107-v4');RUST=Path('/Users/cim310/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc');sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,path,env=None,expected=0):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);path.write_text(json.dumps(d,indent=2)+'\n');print(path.stem,r.returncode,flush=True);assert r.returncode==expected,r.stdout[-3000:]+r.stderr[-3000:];return d
files=lambda p:{str(x.relative_to(p)):sha(x)for x in p.rglob('*')if x.is_file()}
assert files(A/'llvm')==files(A/'llvm-decoded')
base=json.loads(P.joinpath('generate.build.json').read_text())['command']
results={}
for name in ['llvm','llvm-decoded','default']:
 p=A/name;out=P/(name+'-qualification');out.mkdir(exist_ok=True)
 cmd=base[:];cmd[-3]=str(p/'build.rs');cmd[-1]=str(out/'build-script');cmd+=['--extern','sha2=/private/tmp/mech-related-retained-deps/libsha2-f9316069bed82edc.rlib'];run(cmd,out/'build-script-build.json')
 env={'HOST':'aarch64-apple-darwin','TARGET':'aarch64-apple-darwin','CARGO_MANIFEST_DIR':str(p)};run([out/'build-script'],out/'build-script-check.json',env)
 original=p.joinpath('src/main.rs').read_bytes();p.joinpath('src/main.rs').write_bytes(original+b'\n// invalid generated-input edit\n');r=subprocess.run([out/'build-script'],capture_output=True,text=True,env={**os.environ,**env});p.joinpath('src/main.rs').write_bytes(original);assert r.returncode!=0 and 'changed generated input src/main.rs'in r.stderr;out.joinpath('build-script-reject.json').write_text(json.dumps(dict(exit=r.returncode,stdout=r.stdout,stderr=r.stderr,original_restored=True),indent=2)+'\n')
 cmd=base[:];cmd[-3]=str(p/'src/main.rs');cmd[-1]=str(out/'native-app');cmd[1:1]=['-L','dependency='+str(RT)]
 for i,arg in enumerate(cmd):
  if arg.startswith('mech_gpu='):cmd[i]='mech_gpu='+str(RT/'libmech_gpu.rlib')
 cmd+=['-C','link-arg='+str(p/'program.o')];run(cmd,out/'app-build.json')
 run(['python3',R/'audit-native-link.py','--object',p/'program.o','--executable',out/'native-app','--output',out/'native-link-proof.json'],out/'link-audit.json')
 values={}
 for case in ['zero','forty-one','near-max','overflow','zero-work']:
  inp=A/(('forty-one'if case=='zero-work'else case)+'.json');args=['0'if case=='zero-work'else'1000000','1048576','1000000','1000000','1048576','--',str(inp)];d=run([out/'native-app',*args],out/(case+'-run.json'),expected=1 if case in ['overflow','zero-work']else 0);values[case]=(d['exit'],d['stdout'],d['stderr'])
  if case not in ['overflow','zero-work']:assert d['stdout']==(A/(case+'.expected')).read_text()
  else:assert d['stdout']==''
 results[name]=values
assert results['llvm']==results['llvm-decoded']==results['default']
assert files(A/'llvm')==files(A/'llvm-decoded')
P.joinpath('summary.json').write_text(json.dumps(dict(original_decoded_generated_projects_identical=True,generated_main_unmodified=True,generated_build_scripts_executed=3,generated_pin_corruptions_rejected=3,runtime_only_compiler_features=True,LLVM_Cranelift_results_exact=True,successes=9,denials=6,all_same_Object_link_audits_passed=True,Cargo_project_not_built=True,performance_measured=False),indent=2)+'\n')
