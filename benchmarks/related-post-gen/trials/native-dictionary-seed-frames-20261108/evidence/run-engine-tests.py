from pathlib import Path
import os,json,subprocess,time,importlib.util,sys
p=Path(__file__).parent;t=p/sys.argv[1];t.mkdir(exist_ok=False);s=Path('/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108');old='/private/tmp/mech-related-borrowed-map-key-views-compiler-20261108';base=json.loads(Path('/private/tmp/mech-related-borrowed-map-key-continuations-20261108/engine-test-build.json').read_text());cmd=[x.replace(old,str(s))for x in base['command']];cmd[-1]=str(t/'engine-tests');env={k:v.replace(old,str(s))for k,v in base['environment'].items()}
for i,x in enumerate(cmd):
 if x.startswith('metadata='):cmd[i]='metadata=related_native_map_seed_engine_tests_v1'
spec=importlib.util.spec_from_file_location('h','/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(s);(t/'source-inventory.json').write_text(json.dumps(dict(root=str(s),files=files,scope='NativeMapSeedFrame owned API prototype, native seed bypass not connected'),indent=2)+'\n')
def run(c,n):
 start=time.monotonic();r=subprocess.run(c,cwd=s,env={**os.environ,**env},capture_output=True,text=True);(t/(n+'.json')).write_text(json.dumps(dict(command=c,environment=env,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr,scope='API contract qualification only; no performance claim'),indent=2)+'\n');print(n,r.returncode,flush=True)
 if r.returncode:print(r.stderr[-8000:]+r.stdout[-8000:],flush=True)
 assert r.returncode==0
check=cmd.copy();check[-1]=str(t/'engine-check.rmeta');check+=['--emit=metadata'];run(check,'test-check');run(cmd,'test-build');run([str(t/'engine-tests'),'native_map_seed','--test-threads=1','--nocapture'],'map-seed-api');run([str(t/'engine-tests'),'borrowed_map_keys','string_matrix_key','frame_admission','frame_shape','buffer_frames','closed_constructor','--test-threads=1'],'Engine-parent-contracts')
files_after=h.source_inventory(s);assert {k:v for k,v in files.items()if k.startswith('src/engine/')}=={k:v for k,v in files_after.items()if k.startswith('src/engine/')}
