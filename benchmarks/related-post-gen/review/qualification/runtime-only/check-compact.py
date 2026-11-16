from pathlib import Path
import json,shutil,subprocess,time
T=Path(__file__).parent;U=Path('/private/tmp/mech-related-native-fixed-frame-arenas-runners-20261106-v2');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');p=T/'runtime-compact-runner';p.mkdir();(p/'build').symlink_to(U/'compact-4/build',target_is_directory=True);shutil.copyfile(U/'compact-4/runner.rs',p/'runner.rs')
def run(c,name):
 c=list(map(str,c));start=time.monotonic();r=subprocess.run(c,capture_output=True,text=True);row=dict(command=c,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr);(T/(name+'.json')).write_text(json.dumps(row,indent=2)+'\n');assert r.returncode==0,r.stderr
run(['python3',R/'build-host.py','--compiler-build',T/'program-runtime-build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],'runtime-compact-build')
run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],'runtime-compact-audit')
for case,posts,expected in [('first10',R/'fixtures/posts-first-10.json',Path('/private/tmp/mech-related-object-bound-image-budget-checks-20261104-v4/oracle/related_posts_rust.json')),('full5000',Path('/private/tmp/mech-related-post-rust-local-20261102/posts.json'),Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'))]:
 output=p/(case+'-output.json');run([p/'runner-aot',p/'build/source.mec',posts,output,'1'],'runtime-compact-'+case);assert json.loads(output.read_text())==json.loads(expected.read_text())
(T/'compact-summary.json').write_text(json.dumps(dict(runtime_only_compact_source_first10_full5000_exact=True,native_link_audit_passed=True),indent=2)+'\n');print('Runtime-only native numeric comprehension: first10/full5000 exact',flush=True)
