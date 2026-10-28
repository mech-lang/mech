from pathlib import Path
import hashlib,json,os,subprocess,time
root=Path(__file__).resolve().parent
workspace=Path('/private/tmp/mech-deflate-linear-stack')
build=json.loads((root/'current-build.json').read_text())
base=build['command'][:]
i=base.index('-C');base[i+1]='opt-level=1'
j=base.index('-C',i+2);assert base[j+1].startswith('link-arg=');del base[j:j+2]
base=base[:-3]
for name in ['native','bits']:
 source=workspace/f'hosts/gpu/tests/canonical_artifact_{name}.rs'
 output=root/f'selected-{name}-tests'
 assert not output.exists()
 cmd=base+['--test','--cfg','feature="canonical-native"',str(source),'-o',str(output)]
 start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,'CARGO_MANIFEST_DIR':str(workspace/'hosts/gpu')})
 (root/f'selected-{name}-build.log').write_text(r.stdout+r.stderr)
 (root/f'selected-{name}-build.json').write_text(json.dumps({'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-start,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'libraries':build['libraries']},indent=2)+'\n')
 r.check_returncode()
 cmd=[str(output),'--nocapture','--test-threads=1']
 if name=='native':cmd+=['--skip','all_certified_scalar_primitives_match_ordinary_resident_original_and_decoded']
 start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True)
 (root/f'selected-{name}-run.log').write_text(r.stdout+r.stderr)
 (root/f'selected-{name}-run.json').write_text(json.dumps({'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-start,'binary_sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'skipped_coverage':'The all-primitives test needs not/xor, which this selected catalog deliberately omits.' if name=='native' else None},indent=2)+'\n')
 print(r.stdout,flush=True);r.check_returncode()
