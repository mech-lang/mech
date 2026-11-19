from pathlib import Path
import json,subprocess,hashlib,os,shutil,time,sys
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');Q=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995');D=Path(__file__).resolve().parent/sys.argv[1];D.mkdir(exist_ok=True);assert not (D/'build.jsonl').exists()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();paths=set(subprocess.check_output(['git','diff','--name-only'],cwd=W,text=True).splitlines())|set(subprocess.check_output(['git','ls-files','--others','--exclude-standard'],cwd=W,text=True).splitlines())
for path in sorted(paths):
 dest=D/'source-inputs'/path;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(W/path,dest)
pins={p:sha(W/p) for p in sorted(paths)};(D/'source-identities.json').write_text(json.dumps(pins,indent=2)+'\n')
with (D/'implementation.patch').open('w') as log:subprocess.run(['git','diff','HEAD','--',*sorted(paths)],cwd=W,stdout=log,check=True)
profile={'CARGO_BUILD_JOBS':'1','CARGO_PROFILE_RELEASE_OPT_LEVEL':'3','CARGO_PROFILE_RELEASE_DEBUG':'false','CARGO_PROFILE_RELEASE_LTO':'false','CARGO_PROFILE_RELEASE_CODEGEN_UNITS':'16'}
command=['cargo','+nightly-2026-03-03','build','--locked','--release','--config','profile.release.package.libmimalloc-sys.opt-level=1','--manifest-path',str(W/'benchmarks/mech-deflate/native-profile/Cargo.toml'),'--target-dir',str(Q/'target'),'--message-format=json','--bin','compile_profile']
if len(sys.argv)>2:command+=['--features',sys.argv[2]]
record={'command':command,'profile':profile,'source_base':subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip(),'start_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())};(D/'build-start.json').write_text(json.dumps(record,indent=2)+'\n');t=time.perf_counter()
with (D/'build.jsonl').open('w') as log:status=subprocess.run(command,cwd=W,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|profile,stdout=log,stderr=log)
record|={'seconds':time.perf_counter()-t,'exit_code':status.returncode};assert pins=={p:sha(W/p) for p in pins}
if status.returncode==0:
 source=Q/'target/release/compile_profile';digest=sha(source);dest=D/'compiler'/digest[:16]/'compile_profile';dest.parent.mkdir(parents=True);shutil.copy2(source,dest);record|={'compiler':str(dest),'compiler_sha256':digest}
(D/'build.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record),flush=True);raise SystemExit(status.returncode)
