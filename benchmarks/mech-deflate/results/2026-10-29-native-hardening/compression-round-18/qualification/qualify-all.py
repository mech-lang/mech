from pathlib import Path
import hashlib,json,subprocess,time
root=Path(__file__).resolve().parent
workspace=Path('/private/tmp/mech-deflate-linear-stack')
for script in ['engine-units/build.py','full-components.py','primary/qualify-selected.py','object-audit/build.py','after-witness/build-nested.py','after-witness/build-audit.py','engine-units/nonnative-check.py']:
 print('START',script,flush=True)
 subprocess.run(['python3',str(root/script)],cwd=workspace,check=True)
artifact=Path('/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc')
binary=Path('/private/tmp/mech-live-control-target/release/artifact_review')
command=[str(binary),str(artifact.with_suffix('.mec')),str(artifact)]
a=time.monotonic();p=subprocess.run(command,capture_output=True,text=True,check=False)
(root/'source-artifact.stdout').write_text(p.stdout);(root/'source-artifact.stderr').write_text(p.stderr)
(root/'source-artifact.json').write_text(json.dumps({'command':command,'exit':p.returncode,'seconds':time.monotonic()-a,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(artifact.with_suffix('.mec').read_bytes()).hexdigest(),'artifact_sha256':hashlib.sha256(artifact.read_bytes()).hexdigest(),'compression_executed':False},indent=2)+'\n');p.check_returncode()
print('START primary/scalar-controls.py',flush=True)
subprocess.run(['python3',str(root/'primary/scalar-controls.py')],cwd=workspace,check=True)
for script in ['primary/inclusive-profile.py','partition-profile.py']:
 subprocess.run(['python3',str(root/script)],cwd=workspace,check=True)
print('COMPLETE QUALIFICATION',flush=True)
