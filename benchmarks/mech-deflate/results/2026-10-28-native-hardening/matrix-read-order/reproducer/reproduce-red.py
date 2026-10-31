from pathlib import Path
import gzip,hashlib,json,os,re,subprocess,time
root=Path(__file__).resolve().parent/"red"
root.mkdir(exist_ok=False)
workspace=Path('/private/tmp/mech-deflate-linear-stack')
source=workspace/'hosts/gpu/tests/canonical_artifact_program.rs'
text=source.read_text();skipped=[]
def paths(match):
 relative,kind,name=match.groups()
 path=(source.parent/relative).resolve()
 if name=='linked_qualification':
  skipped.append({'module':name,'source':str(path),'reason':'Linked Cargo subprocess qualification is separate from this existing component cohort.'})
  return ''
 return '#[path = '+json.dumps(str(path))+']\n'+kind+'mod '+name+';'
text=re.sub(r'#\[path = "([^"]+)"\]\n((?:pub )?)mod (\w+);',paths,text)
assert len(skipped)==1
adapted=root/'component-qualification-v2.rs';assert not adapted.exists();adapted.write_text(text)
build=json.loads((Path('/private/tmp/mech-owned-turn-round04-20261028/current-build.json')).read_text());cmd=build['command'][:]
i=cmd.index('-C');cmd[i+1]='opt-level=1';i=cmd.index('-C',i+2);assert cmd[i+1].startswith('link-arg=');del cmd[i:i+2];cmd=cmd[:-3]
old=workspace/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-02/round02-runtime-library-refresh.log.gz'
rows=[json.loads(l) for l in gzip.decompress(old.read_bytes()).decode().splitlines() if l.startswith('{')]
tempfile=next(Path(p) for r in rows if r.get('reason')=='compiler-artifact' and r['target']['name']=='tempfile' for p in r['filenames'] if p.endswith('.rlib'))
assert tempfile.is_file()
rows2=[json.loads(l) for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round04-focused-release.log').read_text().splitlines() if l.startswith('{')]
for name in ['sha2','grapheme']:
 candidates=[r for r in rows2 if r.get('reason')=='compiler-artifact' and r['target']['name']==name and (name!='sha2' or '#sha2@0.11.' in r['package_id'])]
 assert len(candidates)==1,(name,candidates)
 path=next(Path(p) for p in candidates[0]['filenames'] if p.endswith('.rlib'))
 cmd+=['--extern',name+'='+str(path)]
cmd+=['--extern','tempfile='+str(tempfile),'--test','--cfg','feature="canonical-native-program"',str(adapted),'-o',str(root/'component-v2-tests')]
start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,'CARGO_MANIFEST_DIR':str(workspace/'hosts/gpu')})
(root/'component-v2-build.log').write_text(r.stdout+r.stderr)
(root/'component-v2-build.json').write_text(json.dumps({'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-start,'original_source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'adapted_source_sha256':hashlib.sha256(adapted.read_bytes()).hexdigest(),'adaptation':'Only absolute module paths and exclusion of linked subprocess module. All maintained test bodies, values and allowances unchanged.','skipped':skipped,'tempfile':{'path':str(tempfile),'sha256':hashlib.sha256(tempfile.read_bytes()).hexdigest()},'libraries':build['libraries']},indent=2)+'\n');r.check_returncode()
cmd=[str(root/'component-v2-tests'),'word_read_qualification','--nocapture','--test-threads=1']
start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,timeout=300)
(root/'component-v2-run.log').write_text(r.stdout+r.stderr)
(root/'component-v2-run.json').write_text(json.dumps({'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-start,'binary_sha256':hashlib.sha256((root/'component-v2-tests').read_bytes()).hexdigest(),'skipped':skipped},indent=2)+'\n');print(r.stdout+r.stderr,flush=True);r.check_returncode()
