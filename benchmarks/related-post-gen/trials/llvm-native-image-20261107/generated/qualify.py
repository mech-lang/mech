from pathlib import Path
import hashlib,json,os,subprocess,time
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-llvm-native-image-compiler-20261107');O=Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');RT=Path('/private/tmp/mech-related-llvm-native-image-runtime-20261107-v4')
def run(cmd,path,env=None,expected=0):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);path.write_text(json.dumps(d,indent=2)+'\n');print(path.stem,r.returncode,flush=True);assert r.returncode==expected,r.stdout[-3000:]+r.stderr[-3000:];return d
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'generate.rs','--out',P/'generate'],P/'generator-build.json')
run([P/'generate',S,P/'applications'],P/'generate-run.json')
A=P/'applications'
assert (A/'llvm/native/object.o').exists(),list((A/'llvm').rglob('*'))
