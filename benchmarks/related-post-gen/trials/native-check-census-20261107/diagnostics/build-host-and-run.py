from pathlib import Path
import json,os,subprocess,time,hashlib,shutil
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-native-check-census-opt-20261107-v1')
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});(P/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr,diagnostic_timings_excluded=True),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-6000:];return r
first=P/'host-build.json'
if first.exists()and not(P/'host-build-first-missing-catalog.json').exists():shutil.copyfile(first,P/'host-build-first-missing-catalog.json')
s=(P/'runner.rs').read_text();assert s.count('let catalog = linked_catalog().unwrap();')==1;s=s.replace('let catalog = linked_catalog().unwrap();','let catalog = mech_stdlib::source_catalog();');(P/'runner.rs').write_text(s)
build=P/'build-diagnostic.py';text=build.read_text();text=text.replace("(P/'runner.rs').write_text(source)","source=source.replace('let catalog = linked_catalog().unwrap();','let catalog = mech_stdlib::source_catalog();')\n(P/'runner.rs').write_text(source)");build.write_text(text)
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'runner.rs','--out',P/'runner-jit'],'host-build')
run([P/'runner-jit',P/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',P/'output.json','1'],'full5000-run',{'MECH_CHECK_CENSUS_OUTPUT':str(P/'census.json')})
assert json.loads((P/'output.json').read_text())==json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
j=json.loads((P/'census.json').read_text());assert j['rows']and all(r['successes']<=r['attempts']for r in j['rows']);print('5000 output exact;',len(j['rows']),'native counter rows',flush=True)
