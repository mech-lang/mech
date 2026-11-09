from pathlib import Path
import json,hashlib,subprocess,time,os,importlib.util,copy
p=Path(__file__).parent;s=Path('/private/tmp/mech-related-fixed-key-native-loops-compiler-20261108');o=Path('/private/tmp/mech-related-fixed-key-native-loops-engine-20261108-v1');o.mkdir(exist_ok=False)
bp=Path('/private/tmp/mech-related-borrowed-dictionary-values-opt-20261108-v5/build.json');b=json.loads(bp.read_text());old=b['source'];prior=next(x for x in b['stages']if x['crate']=='mech_engine');cmd=[a.replace(old,str(s))for a in prior['command']];cmd[-1]=str(o/'libmech_engine.rlib');cmd[1:1]=['-L','dependency='+str(o)]
for i,a in enumerate(cmd):
 if a.startswith('metadata='):cmd[i]='metadata=related_fixed_key_native_loop_engine_foundation_v1'
env={k:v.replace(old,str(s))for k,v in prior['environment'].items()}
spec=importlib.util.spec_from_file_location('h','/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(s);component={k:v for k,v in files.items()if k.startswith('src/engine/')};(o/'actual-source-inventory-at-engine-start.json').write_text(json.dumps(dict(root=str(s),files=files,Engine_component_only_used=True,GPU_may_be_edited_independently=True),indent=2)+'\n')
start=time.monotonic();r=subprocess.run(cmd,cwd=s,env={**os.environ,**env},capture_output=True,text=True);record=dict(crate='mech_engine',command=cmd,environment=env,exit=r.returncode,seconds=time.monotonic()-start,reused=False,component_scope='src/engine/** Rust/manifests; other Mech components are linked pinned dependencies, not compiled inputs',component_source_sha256=hashlib.sha256(json.dumps(component,sort_keys=True).encode()).hexdigest(),stdout=r.stdout,stderr=r.stderr);(o/'mech_engine.build.json').write_text(json.dumps(record,indent=2)+'\n');print('Engine foundation',r.returncode,flush=True)
if r.returncode:print(r.stderr[-8000:],flush=True)
assert r.returncode==0;assert component=={k:v for k,v in h.source_inventory(s).items()if k.startswith('src/engine/')};record['library_sha256']=hashlib.sha256(Path(cmd[-1]).read_bytes()).hexdigest();(o/'mech_engine.build.json').write_text(json.dumps(record,indent=2)+'\n')
# This is explicitly a partially invalidated cache recipe, not a complete compiler.
# Keep exact component inventories, then FORCE every transitive Engine consumer
# to rebuild, even though the Engine itself is now a reusable success.
oldfiles=json.loads((bp.parent/'source-inventory.json').read_text())['files'];hybrid={k:v for k,v in oldfiles.items()if not k.startswith('src/engine/')};hybrid.update(component);source_hash=hashlib.sha256(json.dumps(hybrid,sort_keys=True).encode()).hexdigest();invalid={'mech_engine'};new=[];pending=[]
for stage in b['stages']:
 stage=copy.deepcopy(stage);deps={stage['command'][i+1].split('=',1)[0]for i,a in enumerate(stage['command'])if a=='--extern'}
 if stage['crate']=='mech_engine':stage=record.copy()
 elif deps&invalid:
  invalid.add(stage['crate']);pending.append(stage['crate']);stage.update(exit=1,invalidated_by_new_Engine_dependency=True,previous_success_library_sha256=stage.get('library_sha256'))
 new.append(stage)
b.update(source=str(s),source_sha256=source_hash,stages=new,schema='mech.related-post-gen.dependent-compiler-build.v1',partial_cache_only=True,pending_dependency_rebuilds=pending,cache_policy='Engine compiled/verified against its exact unchanged component; all old dependent libraries explicitly invalidated. Final build-dependent-compiler --repair-failed-stages required. Hybrid inventory describes each recorded compiled component, not a full checkout snapshot.')
(o/'source-inventory.json').write_text(json.dumps(dict(root=str(s),sha256=source_hash,files=hybrid,component_cache_inventory=True),indent=2)+'\n');(o/'build.json').write_text(json.dumps(b,indent=2)+'\n');print('Engine reused foundation ready; forced consumers',pending,flush=True)
