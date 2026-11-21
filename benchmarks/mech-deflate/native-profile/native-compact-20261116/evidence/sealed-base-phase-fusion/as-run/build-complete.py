from pathlib import Path
from hashlib import sha256
import json,os,shutil,subprocess,time
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');L=Path(__file__).resolve().parent;I=L.parent;read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
with (L/'plan.md').open('a') as out:out.write('\nThe four-actor comparison supports the combination: fused+boundary is8/8 faster than original (paired -8.351ms;90%CI[-9.606,-7.931]ms) and8/8 faster than boundary alone (-5.443ms;90%CI[-6.036,-4.225]ms). The original fusion is3.651ms faster in this isolated cohort, unlike the first noisy screen. Keep all trials. Build the production compiler and rerun the full syntax suite; then require identical bytecode/object/full metadata/fixed-support worker and matched complete build timing.\n')
state={str(p.relative_to(W)):sha(p) for p in (W/'src/syntax/src').rglob('*.rs')}
old=L/'tests';test=L/'combined-tests';test.mkdir(exist_ok=True);command=[x.replace(str(old),str(test)) for x in read(old/'build.json')['command']];testenv=env|{'CARGO_MANIFEST_DIR':str(W/'src/syntax'),'CARGO_PKG_VERSION':'0.3.5'}
start=time.perf_counter()
with (test/'build.log').open('w') as log:result=subprocess.run(command,cwd=W,env=testenv,stdout=log,stderr=log)
(test/'build.json').write_text(json.dumps({'command':command,'exit_code':result.returncode,'seconds_diagnostic':time.perf_counter()-start,'source_files':state},indent=2)+'\n');assert result.returncode==0
start=time.perf_counter();args=[str(test/'syntax-tests'),'--test-threads=1']
with (test/'all-tests.log').open('w') as log:result=subprocess.run(args,cwd=W,env=testenv,stdout=log,stderr=log)
(test/'all-tests.json').write_text(json.dumps({'command':args,'binary_sha256':sha(test/'syntax-tests'),'exit_code':result.returncode,'seconds_diagnostic':time.perf_counter()-start,'summary':[line for line in (test/'all-tests.log').read_text().splitlines() if 'test result:' in line]},indent=2)+'\n');assert result.returncode==0
print('Combined candidate: full syntax suite passes',flush=True)
command=['cargo','+nightly-2026-03-03','build','--locked','--offline','--release','--config','profile.release.package.libmimalloc-sys.opt-level=1','--manifest-path','benchmarks/mech-deflate/native-profile/Cargo.toml','--target-dir',str(I/'target'),'--bin','compile_profile','--features','early-region-emission']
env|={'CARGO_BUILD_JOBS':'1','CARGO_PROFILE_RELEASE_OPT_LEVEL':'3','CARGO_PROFILE_RELEASE_DEBUG':'false','CARGO_PROFILE_RELEASE_LTO':'false','CARGO_PROFILE_RELEASE_CODEGEN_UNITS':'16'};start=time.perf_counter()
with (L/'compiler-build.log').open('w') as log:result=subprocess.run(command,cwd=W,env=env,stdout=log,stderr=log)
record={'command':command,'environment':{k:v for k,v in env.items() if k.startswith('CARGO_')},'exit_code':result.returncode,'seconds_diagnostic':time.perf_counter()-start,'source_files':state,'scope':'Compiler bootstrap, not Mech encoder compilation timing.'};(L/'compiler-build.json').write_text(json.dumps(record,indent=2)+'\n');assert result.returncode==0
assert state=={str(p.relative_to(W)):sha(p) for p in (W/'src/syntax/src').rglob('*.rs')}
binary=I/'target/release/compile_profile';pin=sha(binary);dest=L/'compiler'/pin[:16]/'compile_profile';dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(binary,dest);record|={'compiler':str(dest),'compiler_sha256':pin};(L/'compiler-build.json').write_text(json.dumps(record,indent=2)+'\n');print('Immutable compiler built',pin,flush=True)
