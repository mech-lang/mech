from pathlib import Path
from hashlib import sha256
import json,shutil,gzip
L=Path(__file__).resolve().parent;F=L/'final-integration';P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');E=P/'evidence/encoder-size-gap/immutable-current-read-study';read=lambda p:json.loads(p.read_text())
def copy(src,dest):
 data=src.read_bytes();out=E/dest;out.parent.mkdir(parents=True,exist_ok=True)
 if src.suffix=='.rs' or len(data)>45000:out=Path(str(out)+'.gz');out.write_bytes(gzip.compress(data,mtime=0))
 else:out.write_bytes(data)
for f in (F/'source-inputs').rglob('*'):
 if f.is_file():copy(f,str(f.relative_to(L)))
for name in ['source.json','compiler.json','format.json','module-layout.json','warning-policy.json','unsafe-boundary.json','syntax-tests.json','syntax-tests.log','qualification-tests.log','named-rebuild/rebuild.json','named-rebuild/correctness.json','named-rebuild/checked-failures.json','named-rebuild/checked-recoveries.json','matched-builds/plan.json','matched-builds/comparison.json','matched-builds/summary.json','fixed-support-relink/runtime.json','fixed-support-relink/qualification-reuse.json','portable-source-replay/source-reproduction.json']:
 copy(F/name,'final-integration/'+name)
for f in (L/'integration-layout-focused-tests').glob('*'):
 if f.suffix in ['.json','.rs','.log']:copy(f,'integration-layout-focused-tests/'+f.name)
shutil.copy2(E/'source-reproduction.json',E/'placement/source-reproduction.json');shutil.copy2(F/'portable-source-replay/source-reproduction.json',E/'source-reproduction.json')
final=read(F/'compiler.json')
for name in ['candidate.json','emission.json']:
 p=E/name;d=read(p);d['initial_compiler_sha256']=d['compiler_sha256'];d['initial_source_integration_commit']=d['source_integration_commit'];d.update(compiler_sha256=final['compiler_sha256'],source_integration_commit=final['source_commit'],source_inputs=final['source_inputs']);p.write_text(json.dumps(d,indent=2)+'\n')
for f in L.glob('*.py'):shutil.copy2(f,E/'as-run'/f.name)
