from pathlib import Path
import copy,hashlib,json,os,shutil,subprocess,time
T=Path(__file__).parent;O=Path('/private/tmp/mech-related-owned-shape-metadata-opt-20261107-v1');U=Path('/private/tmp/mech-related-owned-shape-metadata-runners-20261107-v1');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');d=json.loads((O/'build.json').read_text());S=Path(d['source']);stages={s['crate']:s for s in d['stages']};libs={k:s['command'][-1]for k,s in stages.items()};sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(cmd,name,env=None):
 cmd[:]=list(map(str,cmd))
 if '-o'in cmd:
  i=cmd.index('-o');output=cmd.pop(i+1);cmd.pop(i);cmd.extend(['-o',output])
 start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});row=dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr,compiler_source_sha256=d['source_sha256']);(T/(name+'.json')).write_text(json.dumps(row,indent=2)+'\n');assert r.returncode==0,r.stderr;print(name,'passed',flush=True);return row
for label,features in [('scalar-runtime',['canonical-native-runtime']),('program-runtime',['canonical-native-runtime','canonical-native-program-runtime'])]:
 stage=stages['mech_gpu'];cmd=[];i=0
 while i<len(stage['command']):
  a=stage['command'][i]
  if a=='--cfg':i+=2;continue
  cmd.append(a);i+=1
 cmd[-1]=str(T/('libmech_gpu_'+label.replace('-','_')+'.rlib'))
 for f in features:cmd.extend(['--cfg','feature="'+f+'"'])
 for i,a in enumerate(cmd):
  if a.startswith('metadata='):cmd[i]=a+'_'+label.replace('-','_')
 run(cmd,label,stage['environment'])
 if label=='program-runtime':
  runtime_build=copy.deepcopy(d);g=next(s for s in runtime_build['stages']if s['crate']=='mech_gpu');g.update(command=cmd,library_sha256=sha(cmd[-1]),exit=0);(T/'program-runtime-build.json').write_text(json.dumps(runtime_build,indent=2)+'\n');p=T/'runtime-runner';p.mkdir();(p/'build').symlink_to(U/'after-4/build',target_is_directory=True);shutil.copyfile(U/'after-4/runner.rs',p/'runner.rs');run(['python3',R/'build-host.py','--compiler-build',T/'program-runtime-build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],'runtime-runner-build');run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],'runtime-runner-audit')
  for case,posts,oracle in [('first10',R/'fixtures/posts-first-10.json',Path('/private/tmp/mech-related-object-bound-image-budget-checks-20261104-v4/oracle/related_posts_rust.json')),('full5000',Path('/private/tmp/mech-related-post-rust-local-20261102/posts.json'),Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'))]:
   output=p/(case+'-output.json');run([p/'runner-aot',p/'build/source.mec',posts,output,'1'],'runtime-'+case);assert json.loads(output.read_text())==json.loads(oracle.read_text())
# Build genuine current runtime image support and the application generator.
rt=stages['mech_runtime'];cmd=rt['command'][:];cmd[-1]=str(T/'libmech_runtime_images.rlib');cmd.extend(['--cfg','feature="canonical-native-images"','--extern','serde_json=/private/tmp/mech-related-retained-deps/libserde_json-e0f8af30c3327a5b.rlib']);cmd[cmd.index('--crate-name')+1]='mech_runtime'
for i,a in enumerate(cmd):
 if a.startswith('metadata='):cmd[i]=a+'_images'
run(cmd,'runtime-images',rt['environment']);rtcmd=cmd[:]
gpu=stages['mech_gpu'];cmd=gpu['command'][:];cmd[-1]=str(T/'libmech_gpu_hosted.rlib');cmd.extend(['--cfg','feature="canonical-native-program-host"'])
for i,a in enumerate(cmd):
 if a=='--extern'and cmd[i+1].startswith('mech_runtime='):cmd[i+1]='mech_runtime='+str(T/'libmech_runtime_images.rlib')
 if a.startswith('metadata='):cmd[i]=a+'_hosted'
run(cmd,'hosted-generation',gpu['environment']);hosted=copy.deepcopy(d)
for s in hosted['stages']:
 if s['crate']=='mech_runtime':s.update(command=rtcmd,library_sha256=sha(rtcmd[-1]),exit=0)
 if s['crate']=='mech_gpu':s.update(command=cmd,library_sha256=sha(cmd[-1]),exit=0)
(T/'hosted-build.json').write_text(json.dumps(hosted,indent=2)+'\n');shutil.copyfile('/private/tmp/mech-related-borrowed-matrix-map-keys-executable-opt-20261105-v1/generate-standard-app.rs',T/'generate-standard-app.rs');run(['python3',R/'build-host.py','--compiler-build',T/'hosted-build.json','--source',T/'generate-standard-app.rs','--out',T/'generate-standard-app'],'generator-build');app=T/'managed-app';run([T/'generate-standard-app',S,app],'generate-standard-app')
# Compile the unmodified generated main against the normal production closure.
bundle=app/'audit-bundle';bundle.mkdir();mapping={'application.mec':'source.mec','program.mecb':'artifact.mecb','program.o':'object.o','native-descriptor.json':'descriptor.json','native-descriptor.bin':'legacy-descriptor.bin'}
for old,new in mapping.items():shutil.copyfile(app/old,bundle/new)
manifest=json.loads((app/'linked-manifest.json').read_text());(bundle/'metadata.json').write_text(json.dumps(manifest['object'],indent=2)+'\n');(bundle/'linked.rs').write_text('// Native symbols are bound directly by the generated main.\n')
for file,h in manifest['pins'].items():assert sha(app/file)==h,(file,h)
import importlib.util,zlib
app=T/'managed-app';bundle=app/'audit-bundle';manifest=json.loads((app/'linked-manifest.json').read_text())
spec=importlib.util.spec_from_file_location('audit_native_link',R/'audit-native-link.py');audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
ob,sections,symbols=audit.macho(bundle/'object.o');metadata=manifest['object'];symbol=next(s for s in symbols if s['name']=='_'+metadata['native_metadata_symbol']);section=sections[symbol['section']-1];offset=section['offset']+symbol['value']-section['address'];size=metadata['native_metadata_bytes'];raw=ob[offset:offset+size];assert raw[:8] in [b'MECHTVI1',b'MECHTVI2',b'MECHTVI3',b'MECHTVI4',b'MECHTVI5'] and size==13+int.from_bytes(raw[9:13],'little')
packed=(app/'native-descriptor.bin').read_bytes();assert packed[:8]==b'MECHNDI\x01' and zlib.decompress(packed[24:])==(app/'native-descriptor.json').read_bytes()
(bundle/'native-metadata.bin').write_bytes(raw)
(T/'managed-audit-input-correction.json').write_text(json.dumps(dict(reason='Generated legacy packed JSON is not the Object-owned typed metadata. Audit exact typed export bytes and linked binding, preserve and verify legacy pins separately.',legacy_packed_json_sha256=sha(app/'native-descriptor.bin'),typed_metadata_sha256=sha(bundle/'native-metadata.bin'),typed_metadata_bytes=size,object_sha256=sha(bundle/'object.o')),indent=2)+'\n')

main_hash=sha(app/'src/main.rs');run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',app/'src/main.rs','--out',app/'runner-standard','--bundle',bundle,'--linked'],'managed-main-build');run(['python3',R/'audit-native-link.py','--object',bundle/'object.o','--executable',app/'runner-standard','--output',app/'native-link-audit.json'],'managed-main-audit');assert sha(app/'src/main.rs')==main_hash
old=Path('/private/tmp/mech-borrowed-matrix-map-keys-standard-app-20261105-v2');expected=json.loads((R/'results/borrowed-matrix-map-keys-executable-20261105/managed-app/summary.json').read_text())['expected_canonical_snapshot'];cases=[]
for case,cap,exit_code in [('success','1000000000000',0),('overflow','1000000000000',1),('work-denial','0',1)]:
 shutil.copyfile(old/(case+'-input.json'),app/(case+'-input.json'));c=[str(app/'runner-standard'),cap,*(['1000000000000']*4),'--',str(app/(case+'-input.json'))];r=subprocess.run(c,capture_output=True,text=True);assert r.returncode==exit_code,(case,r.stderr)
 if case=='success':assert r.stdout==expected
 elif case=='overflow':assert 'Arithmetic'in r.stderr
 else:assert 'WorkLimit'in r.stderr
 cases.append(dict(case=case,command=c,exit=r.returncode,stdout=r.stdout,stderr=r.stderr))
summary=dict(status='passed',runtime_only_scalar_and_program_builds=True,runtime_only_full5000_exact_rust=True,generated_main_executed_unmodified=True,manifest_pins_exact=True,generated_main_sha256=main_hash,actual_native_object_exports_audited=True,cases=cases,compiler_build_sha256=sha(O/'build.json'));(T/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print('Runtime-only/full5000 and unmodified Managed application qualification passed',flush=True)
