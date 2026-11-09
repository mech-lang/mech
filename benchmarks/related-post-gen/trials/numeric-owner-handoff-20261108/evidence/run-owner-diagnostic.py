from pathlib import Path
import os,json,subprocess,time,hashlib,importlib.util
P=Path(__file__).parent;O=Path('/private/tmp/mech-related-numeric-owner-handoff-diagnostic-opt-20261108-v1');S=Path('/private/tmp/mech-related-numeric-owner-handoff-compiler-20261108');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');N=P/'diagnostic-host'
read=lambda p:json.loads(Path(p).read_text())
def run(c,name,env=None):
 c=list(map(str,c));t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True,env={**os.environ,**(env or {})});(N/(name+'.json')).write_text(json.dumps(dict(command=c,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,diagnostic_only=True,all_timings_ineligible=True),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stderr[-4000:];return r
spec=importlib.util.spec_from_file_location('h',P/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S);assert files==read(O/'source-inventory.json')['files'];b=read(O/'build.json');assert len(b['stages'])==12 and all(s['exit']==0 for s in b['stages'])
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',N/'runner.rs','--out',N/'runner-owner-diagnostic','--bundle',N/'build','--linked'],'host-build')
run(['python3',R/'audit-native-link.py','--object',N/'build/object.o','--executable',N/'runner-owner-diagnostic','--output',N/'native-link-proof.json'],'audit-run');a=read(N/'native-link-proof.json');old=read('/private/tmp/mech-related-fixed-key-native-loops-runners-20261108-v1/histogram-reuse/native-link-proof.json');assert a['normalized_text_sha256']==old['normalized_text_sha256']
r=run([N/'runner-owner-diagnostic',N/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',N/'output.json','1'],'owner-diagnostic',{'MECH_DIAGNOSE_NUMERIC_OWNER':'1'})
assert read(N/'output.json')==read('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json');(N/'ownership-trace.txt').write_text(r.stderr)
assert files==h.source_inventory(S)
print(r.stderr,flush=True)
