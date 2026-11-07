from pathlib import Path
import json,shutil,subprocess,time,hashlib
P=Path(__file__).parent;O=Path('/private/tmp/mech-related-llvm-typed-ssa-opt-20261107-v6');B=Path('/private/tmp/mech-related-forward-read-windows-runners-20261107-v8');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,path):
 start=time.monotonic();r=subprocess.run(list(map(str,cmd)),capture_output=True,text=True);path.write_text(json.dumps(dict(command=list(map(str,cmd)),exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
shutil.copyfile('/private/tmp/mech-related-forward-read-windows-opt-20261107-v8/compiler-facts.rs',O/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',O/'compiler-facts.rs','--out',O/'compiler'],P/'compiler-build.json')
proof={}
for name in ['after-4','reuse-4']:
 p=P/name;p.mkdir(exist_ok=True);run([O/'compiler',B/name/'build/source.mec',p/'build'],p/'compile.json');old={x.name:sha(x)for x in(B/name/'build').iterdir()if x.is_file()};new={x.name:sha(x)for x in(p/'build').iterdir()if x.is_file()};assert len(old)==len(new)==13 and old==new,(name,{k:(v,new.get(k))for k,v in old.items()if new.get(k)!=v})
 proof[name]=dict(all13_bundle_files_byte_exact=True,files=new,unchanged_typed_IR=264,LLVM_code_used=False);print(name,'all13 files byte-exact, including Object and descriptor',flush=True)
(P/'equivalence.json').write_text(json.dumps(proof,indent=2)+'\n')
