from pathlib import Path
from hashlib import sha256
import json,shutil,sys
sys.path.insert(0,str(Path(__file__).resolve().parent.parent.parent/'current-roadmap/scorer-runtime'));import emit
L=Path(__file__).resolve().parent/'immutable-only';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
build=read(L/'compiler-build.json');compiler=Path(build['compiler']);shutil.copy2(L/'compiler-build.json',compiler.with_name('result.json'))
for role in ['disabled-control','immutable-defaults','immutable-defaults-checked']:
 p=L/role/'receipt.json';r=read(p);r|={'layout':emit.symbols(L/role/'frontend/program.o'),'source_commit':r['source_base'],'source_pin':r['source_base'],'object_sha256':r['artifact_sha256']['program.o'],'metadata_sha256':r['artifact_sha256']['metadata.json'],'bytecode_sha256':r['artifact_sha256']['program.mecb'],'artifacts':r['artifact_sha256']};p.write_text(json.dumps(r,indent=2)+'\n')
print('Bound compiler result and complete artifacts to every source receipt.')
