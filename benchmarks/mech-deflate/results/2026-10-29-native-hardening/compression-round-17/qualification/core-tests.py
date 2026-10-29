from pathlib import Path
import json,subprocess,hashlib,time
r=Path(__file__).resolve().parent;w=Path('/private/tmp/mech-deflate-linear-stack')
rows=[json.loads(l) for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round17-prepared-descriptors-release-final.log').read_text().splitlines() if l.startswith('{')]
assert rows[-1]=={'reason':'build-finished','success':True}
core=next(Path(p) for a in rows if a.get('reason')=='compiler-artifact' and a['target']['name']=='mech_core' for p in a['filenames'] if p.endswith('.rlib'))
c=['/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc','--edition=2024','--test','-C','opt-level=1','-D','warnings','-L','dependency=/private/tmp/mech-live-control-target/release/deps','--extern','mech_core='+str(core),str(w/'src/core/tests/type_system_contract.rs'),'-o',str(r/'core-tests')]
a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True);(r/'core-build.log').write_text(p.stdout+p.stderr);(r/'core-build.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a,'core_sha256':hashlib.sha256(core.read_bytes()).hexdigest()},indent=2)+'\n');print(p.stderr[-2000:]);p.check_returncode()
c=[str(r/'core-tests'),'--nocapture','--test-threads=1'];a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True);(r/'core-run.log').write_text(p.stdout+p.stderr);(r/'core-run.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a,'binary_sha256':hashlib.sha256((r/'core-tests').read_bytes()).hexdigest()},indent=2)+'\n');print(p.stdout[-1500:]+p.stderr[-1500:]);p.check_returncode()
