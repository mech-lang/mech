"""Archive all bounded literal-reuse experiments without promoting an unqualified worker."""
from pathlib import Path
from hashlib import sha256
import json,gzip,shutil,tarfile,io
L=Path(__file__).resolve().parent;Q=L.parent.parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');E=W/'benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/encoder-size-gap/immutable-current-read-study';E.mkdir(exist_ok=True);read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(p.read_bytes()).hexdigest()
def copy(source,dest=None,required=True):
 p=L/source
 if not p.is_file():
  assert not required,source
  return
 q=E/(dest or source);q.parent.mkdir(parents=True,exist_ok=True)
 if p.suffix in ['.rs','.patch','.clif'] or p.stat().st_size>45000:q=Path(str(q)+'.gz');q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
 else:shutil.copy2(p,q)
for name in ['plan.md','pinned-runtime-plan.json','profiled-work-comparison.json','current-byte-loads.json','index-phi-analysis.json','selected-hot-block-work.json','repair/read-sharing-diagnosis.json','repair/byte-site-comparison.json']:
 copy(name)
for stage in ['','repair/','pinned/','immutable-only/','placement/']:
 for name in ['compiler-build.json','compiler-build.log','source.patch','layout-proof.json','matcher-layout.json']:
  copy(stage+name,required=False)
for tests in ['gpu-tests','pinned-gpu-tests','focused-tests','focused-tests-first-prototype','immutable-only-focused-tests','placement-focused-tests']:
 for name in ['build.json','build.log','tests.rs','tests.log','result.json','defaulted-loads.json','private-loan.json','private-contracts.json']:
  copy(tests+'/'+name,required=False)
for group in ['first-prototype-source','repair/source-before-pinning','pinned/source-inputs','pinned/source-before-immutable-only','immutable-only/source-inputs','placement/source-inputs']:
 for f in (L/group).rglob('*'):
  if f.is_file() and f.suffix in ['.rs','.json','.log']:copy(str(f.relative_to(L)))
for role in ['control','safe-addresses','immutable-defaults','repair/immutable-defaults','pinned/immutable-defaults','repair/immutable-profiled','pinned/immutable-profiled','immutable-only/immutable-defaults','immutable-only/immutable-profiled','placement/immutable-defaults','placement/immutable-profiled']:
 for name in ['receipt.json','compile.log','matcher-counts/counts.json','matcher-counts/plan.json','bound/runtime.json','bound/qualification.json','bound/smokes.json','bound/smokes.stderr','canonical-counter-input/canonicalization.json','linked-proof.json','recoveries/result.json']:
  copy(role+'/'+name,required=False)
 for name in ['program.o','metadata.json']:
  copy(role+'/frontend/'+name)
 if (L/role/'bound/mech-worker').exists():copy(role+'/bound/mech-worker')
 proof=L/role/'linked-proof.json'
 if proof.is_file():
  record=read(proof);result=Path(record['result']);assert sha(result)==record['sha256'];q=E/role/'linked-checks.json';q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(result.read_bytes())
for stage in ['repair','pinned','placement']:
 folder=stage+'/immutable-profiled/focus'
 if not (L/folder/'screen.json').exists():continue
 for name in ['plan.json','plan.sha256','post-acquisition.json','focus-replay.json','screen.json','environment.json','contract.json','fixture-content.json','acquire.log','replay.log']:
  copy(folder+'/'+name)
 plan=read(L/folder/'plan.json')
 for run in plan['runs']:
  actual=Q/run['id'];order=run['order'];out=E/folder/order;out.mkdir(exist_ok=True)
  for name in ['pins.json','complete.json','summary.json','mech-control.stderr','mech-experiment.stderr','roc.stderr','libdeflate.stderr']:
   p=actual/name;assert p.is_file();data=p.read_bytes();(out/(name+'.gz')).write_bytes(gzip.compress(data,mtime=0))
  with tarfile.open(E/folder/(order+'-observations.tar.gz'),'w:gz') as archive:
   for p in sorted(actual.glob('*/observations.json.gz')):
    data=p.read_bytes();info=tarfile.TarInfo(p.parent.name+'/'+p.name);info.size=len(data);info.mtime=0;archive.addfile(info,io.BytesIO(data))
for f in L.glob('*.py'):copy(f.name,'as-run/'+f.name)
for name in ['unadmitted-private-fixture.mec','private_phase_tests-before-fixture-repair.rs']:
 copy(name,required=False)
copy('repair/inspect-cse/mech_compact_numeric_475_vector.clif',required=False)
reference=Q/'consolidation-995/natural4k-runtime-target/hot-matcher-facts/profiled-matcher-counts/counts.json';(E/'selected-reference-counters.json.gz').write_bytes(gzip.compress(reference.read_bytes(),mtime=0))
print('Archived',len([p for p in E.rglob('*') if p.is_file()]),'source/evidence files.',flush=True)

for name in ['production-integration.json','placement/result.json','placement/runtime-case-ratios.json','placement/required-wins.json','placement/remaining-losses.json','placement/matched-builds/plan.json','placement/matched-builds/comparison.json','placement/matched-builds/summary.json','placement/native-detail/result.json','placement/native-sample/result.json','placement/native-sample/sample.txt','placement/native-sample/sample.log','placement/native-sample/compile.log']:
 copy(name)
for stage in ['repair','pinned','immutable-only','placement']:
 for name in ['checker-match.json','compile.log']:
  copy(stage+'/immutable-profiled-checked/'+name,required=False)
for f in (L/'placement/current-runtime-profiles').rglob('*'):
 if f.is_file():copy(str(f.relative_to(L)))
folder='placement/immutable-profiled/full'
for name in ['plan.json','plan.sha256','post-acquisition.json','full-replay.json','screen.json','environment.json','contract.json','fixture-content.json','acquire.log','replay.log']:
 copy(folder+'/'+name)
for run in read(L/folder/'plan.json')['runs']:
 actual=Q/run['id'];order=run['order'];out=E/folder/order;out.mkdir(exist_ok=True)
 for name in ['pins.json','complete.json','summary.json','mech-control.stderr','mech-experiment.stderr','roc.stderr','libdeflate.stderr']:
  p=actual/name;assert p.is_file();(out/(name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
 with tarfile.open(E/folder/(order+'-observations.tar.gz'),'w:gz') as archive:
  for p in sorted(actual.glob('*/observations.json.gz')):
   data=p.read_bytes();info=tarfile.TarInfo(p.parent.name+'/'+p.name);info.size=len(data);info.mtime=0;archive.addfile(info,io.BytesIO(data))
from runpy import run_path
import sys
script=Q/'current-roadmap/shared-terminal-returns/measure.py';sys.path.insert(0,str(script.parent));tools=run_path(str(script),run_name='record_build_timer');timer=tools['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==read(L/'placement/matched-builds/plan.json')['timer_sha256'];shutil.copy2(timer,E/'as-run/build-timer.py');shutil.copy2(Q/'compare-library.py',E/'as-run/runtime-timer.py')
print('Final archive includes all actual full orders, complete-build observations and current profiles.',flush=True)
