"""Publish original observations and plans without compiler/counter binaries."""
from pathlib import Path
from hashlib import sha256
import gzip,json,shutil,subprocess
L=Path(__file__).resolve().parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');P=W/'benchmarks/mech-deflate/native-profile/native-compact-20261116';O=P/'evidence/sealed-base-phase-fusion';O.mkdir(exist_ok=True)
plain=['plan.md','plan.json','fusion-source.json','repair-source.json','profile-summary.json','census-summary.json','screen-plan.json','screen/results.json','screen-fused-bounds/results.json','factorial/results.json','factorial/plan.json','fused/exact-smoke.json','fused/build.json','fused-bounds/build.json','bounds-only/build.json','combined-tests/build.json','combined-tests/all-tests.json','tests/build.json','tests/all-tests.json','control/build.json','census/build.json','probe-build.json','compiler-build.json','bootstrap-cache-failure/build.json','bootstrap-cache-failure/clean.json','exact-artifacts.json','portable-source-replay/source-reproduction.json','portable-source-replay-command.json','matched-builds/plan.json','matched-builds/summary.json']
compressed=['profile-captures.json','sample-0/sample.txt','sample-1/sample.txt','sample-2/sample.txt','census/run-0.log','census/run-1.log','census/run-0.0.json','census/run-1.0.json','control/run-0.0.json','control/run-1.0.json','combined-tests/all-tests.log','combined-tests/build.log','tests/all-tests.log','tests/fused-test.log','portable-source-replay.log','compiler-build.log','bootstrap-cache-failure/build.log','bootstrap-cache-failure/clean.log','emitted/compile.log','emitted/bound/runtime.json','matched-builds/comparison.json']
for name in plain+compressed:
 src=L/name
 assert src.exists(),name
 dst=O/(name+'.gz' if name in compressed else name);dst.parent.mkdir(parents=True,exist_ok=True)
 if name in compressed:dst.write_bytes(gzip.compress(src.read_bytes(),mtime=0))
 else:shutil.copyfile(src,dst)
for name in ['build-census.py','build-variant.py','build-complete.py','resume-compiler.py','profile-current.py','screen.py','factorial.py','measure-complete.py','implement-fusion.py','parser-probe.rs','publish.py','inspect-collector-first.py','inspect-collector-vcode.py']:
 dst=O/'as-run'/name;dst.parent.mkdir(exist_ok=True);shutil.copyfile(L/name,dst)
source=O/'variants';source.mkdir(exist_ok=True)
for name in ['continuation-before.rs','fusion-alone.rs','fusion-alone.patch']:
 shutil.copyfile(L/name,source/name)
shutil.copyfile(W/'src/syntax/src/document/parser/canonical/base/continuation.rs',source/'combined.rs')
shutil.copyfile(L/'bounds-only-source/document/parser/canonical/base/continuation.rs',source/'bounds-only.rs')
shutil.copyfile(W/'src/syntax/src/document/parser/canonical/base/continuation/fused_tests.rs',source/'fused_tests.rs')
(source/'combined.patch').write_text(subprocess.check_output(['git','diff','--','src/syntax/src/document/parser/canonical/base/continuation.rs'],cwd=W,text=True))
# The unchanged native bytes and runtime receipts already live in the previous package.
previous=P/'evidence/encoder-size-gap/current-matcher-guard-facts';paths=['candidate.json','full/full-replay.json.gz','full/screen.json','guards-profiled/program.o.gz','guards-profiled/metadata.json.gz','guards-profiled/program.mecb.gz','guards-profiled/mech-worker.gz','guards-profiled/runtime.json']
linked={}
for path in paths:
 src=previous/path
 if not src.exists():
  print('Cannot find prior evidence pin:',src,flush=True);continue
 linked[str(src.relative_to(P))]=sha256(src.read_bytes()).hexdigest()
(O/'reused-runtime-evidence.json').write_text(json.dumps({'scope':'Reuse only for the byte-identical worker reproduced in exact-artifacts.json; no new runtime campaign claimed.','files':linked},indent=2)+'\n')
runtime=L/'current-runtime-profiles'
assert (runtime/'summary.json').exists()
for src in runtime.rglob('*'):
 if not src.is_file():continue
 relative=src.relative_to(runtime);dst=O/'current-runtime-profiles'/relative
 if src.suffix in ['.txt','.log']:dst=dst.with_name(dst.name+'.gz');dst.parent.mkdir(parents=True,exist_ok=True);dst.write_bytes(gzip.compress(src.read_bytes(),mtime=0))
 else:dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dst)
shutil.copyfile(L/'profile-selected-runtime.py',O/'as-run/profile-selected-runtime.py')
for name in ['collector-inspection','collector-inspection-vcode']:
 srcroot=L/name
 for relative in ['plan.json','failed-inspection.json','body-check.json','result.json','compile.log','mech_compact_numeric_475_vector.clif','mech_compact_numeric_475_vector.vcode','mech_compact_numeric_475_vector.proof.txt','frontend/program.o','frontend/metadata.json']:
  src=srcroot/relative
  if not src.exists():continue
  compressed=src.suffix in ['.txt','.log','.clif','.vcode','.o'] or relative=='frontend/metadata.json';dst=O/name/(relative+'.gz' if compressed else relative);dst.parent.mkdir(parents=True,exist_ok=True)
  if compressed:dst.write_bytes(gzip.compress(src.read_bytes(),mtime=0))
  else:shutil.copyfile(src,dst)
print('Published original observations and source variants',flush=True)
