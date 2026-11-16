from pathlib import Path
import json,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-deflate-reduction-integration-opt-20261106-v1')
roots={223: '/private/tmp/mech-related-native-fixed-frame-arenas-runners-20261106-v2/after-4/build', 224: '/private/tmp/mech-related-native-boolean-bit-tests-runners-20261106-v1/after-4/build', 225: '/private/tmp/mech-related-native-fuel-derived-debits-runners-20261106-v2/after-4/build', 226: '/private/tmp/mech-related-latest-deflate-integration-runners-20261106-v1/after-4/build'}
roots[227]='/private/tmp/mech-related-latest-deflate-integration-runners-20261106-v5/after-4/build'
for ir,path in roots.items():
 source=Path(path);assert json.loads((source/'metadata.json').read_text())['typed_ir_version']==ir
 p=U/str(ir);p.mkdir();(p/'build').symlink_to(source,target_is_directory=True);(p/'probe.rs').write_text((U/'probe.rs').read_text().replace('"old_typed_ir":197','"old_typed_ir":'+str(ir)))
 for name,cmd in [('build',['python3',str(R/'build-host.py'),'--compiler-build',str(O/'build.json'),'--source',str(p/'probe.rs'),'--out',str(p/'probe'),'--bundle',str(p/'build'),'--linked']),('run',[str(p/'probe')])]:
  t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);(p/(name+'.json')).write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stderr;print(ir,name,'passed',flush=True)
