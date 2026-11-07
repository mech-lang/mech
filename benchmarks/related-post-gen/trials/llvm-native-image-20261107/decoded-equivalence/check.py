from pathlib import Path
import hashlib,json,subprocess,time
P=Path(__file__).parent;B=Path('/private/tmp/mech-related-llvm-native-image-smoke-20261107-v4');O=Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,path):
 t=time.monotonic();cmd=list(map(str,cmd));r=subprocess.run(cmd,capture_output=True,text=True);path.write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(path.stem,r.returncode,flush=True);assert r.returncode==0,r.stdout[-3000:]+r.stderr[-3000:]
source=(B/'compiler-llvm-image.rs').read_text();needle='let artifact = compile(&source);';assert source.count(needle)==1;source=source.replace(needle,needle+'\n    let artifact=mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap()).unwrap();');(P/'compiler-decoded.rs').write_text(source)
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'compiler-decoded.rs','--out',P/'compiler-decoded'],P/'compiler-build.json')
proof={}
for name in ['after-4','reuse-4']:
 p=P/name;p.mkdir(exist_ok=True);run([P/'compiler-decoded',B/name/'build/source.mec',p/'build'],p/'compile.json');old={f.name:sha(f)for f in(B/name/'build').iterdir()if f.is_file()};new={f.name:sha(f)for f in(p/'build').iterdir()if f.is_file()};assert len(old)==len(new)==13 and old==new,(name,{k:(v,new.get(k))for k,v in old.items()if new.get(k)!=v});proof[name]=dict(all13_bundle_files_byte_exact=True,files=new);print(name,'original/decoded13 files identical',flush=True)
(P/'equivalence.json').write_text(json.dumps(proof,indent=2)+'\n')
