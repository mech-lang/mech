from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');N=Path('/private/tmp/mech-related-completed-key-additions-runners-20261108-v1');RT=P/'profile/runtime';OLD=Path('/private/tmp/mech-related-aligned-constructor-diagnostics-20261108');rows=[]
def read(p):return json.loads(Path(p).read_text())
def write(p,j):Path(p).write_text(json.dumps(j,indent=2)+'\n')
def run(cmd,path):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);write(path,dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,diagnostic_only=True,all_timings_ineligible=True));assert r.returncode==0,r.stderr[-4000:];print(path.parent.name,path.name,flush=True)
shutil.copyfile(OLD/'allocation_counters.rs',P/'profile/allocation_counters.rs');base=read('/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/both-fusion/build/descriptor.json')['descriptor'];origins=[base['calls'][i]['template']['origin']for i in [249,1014]]
variants=[('key-additions-disabled',Path('/private/tmp/mech-related-completed-key-additions-control-20261108-v1/histogram-reuse')),('key-additions-enabled',N/'histogram-reuse')]
for name,production in variants:
 root=P/'profile/hosts'/name;root.mkdir(parents=True,exist_ok=False);(root/'build').symlink_to(production/'build',target_is_directory=True);d=read(root/'build/descriptor.json')['descriptor'];points=[]
 for origin in origins:
  matches=[i for i,x in enumerate(d['calls'])if x and x['template']['origin']==origin and x['template']['selected_key']==dict(module_path=['sequence'],operation_name='with-capacity')];assert len(matches)==1,(name,origin,matches);points.append(matches[0])
 assert len(d['parallel']['regions'])==1;points.append(d['parallel']['regions'][0]['begin'])
 s=Path('/private/tmp/mech-related-native-thread-counters-20261107/candidate/runner.rs').read_text();a='use mech_core::';assert s.count(a)==1;s=s.replace(a,'#[path = "'+str(P/'profile/allocation_counters.rs')+'"] mod allocation_counters;\n#[global_allocator]\nstatic DIAGNOSTIC_ALLOCATOR: allocation_counters::Tracked = allocation_counters::Tracked;\n'+a);a='    let trace = mech_gpu::native_artifact::diagnostic_intervals_finish();';assert s.count(a)==1;s=s.replace(a,a+'\n    assert_eq!(allocation_counters::local_unavailable(),0);');(root/'runner.rs').write_text(s)
 run(['python3',R/'build-host.py','--compiler-build',RT/'build.json','--source',root/'runner.rs','--out',root/'runner-profile','--bundle',root/'build','--linked'],root/'runner-build.json');run(['python3',R/'audit-native-link.py','--object',root/'build/object.o','--executable',root/'runner-profile','--output',root/'native-link-proof.json'],root/'audit-run.json');assert read(root/'native-link-proof.json')['normalized_text_sha256']==read(production/'native-link-proof.json')['normalized_text_sha256'];rows.append(dict(name=name,kind='mech',executable=str(root/'runner-profile'),source=str(root/'build/source.mec'),phase_boundaries=points,source_sha256=hashlib.sha256((root/'build/source.mec').read_bytes()).hexdigest(),Object_sha256=hashlib.sha256((root/'build/object.o').read_bytes()).hexdigest(),Object_text_exact_production=True))
rust=next(x for x in read(OLD/'variants.json')if x['kind']=='rust');rows.append(rust)
for r in rows:r['executable_sha256']=hashlib.sha256(Path(r['executable']).read_bytes()).hexdigest()
write(P/'profile/variants.json',rows);print('All current-native profile hosts built; no capture has run.',flush=True)
