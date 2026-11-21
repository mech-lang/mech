"""Collect all-unit diagnostic counters; these executions are not timings."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,shutil
LAB=Path(__file__).resolve().parent
ROOT=LAB/'early-codebook-construction-census'
ROOT.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
selected=read(LAB/'sparse-guard-rounds/lazy-index/named-source-rebuild/rebuild.json')
phase=read(LAB/'checked-root-phase/source-rebuild/rebuild.json')
binary=LAB/'target/release/compile_profile'
compiler=ROOT/'compiler'/sha(binary)[:16]/'compile_profile';compiler.parent.mkdir(parents=True,exist_ok=True)
if not compiler.exists():shutil.copy2(binary,compiler)
assert sha(compiler)==sha(binary)
patch=subprocess.check_output(['git','diff','--binary','HEAD'])
(ROOT/'compiler-source.patch').write_bytes(patch)
(ROOT/'plan.json').write_text(json.dumps({'compiler_sha256':sha(compiler),'source_base':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'source_patch_sha256':sha(ROOT/'compiler-source.patch'),'feature':'construction-census','purpose':'complete DFG construction work baseline, not latency','cases':['selected-63804','late-codebook-64444'],'omitted_counters':['canonical plan and typed proof table allocations','allocator capacities and scratch maps','semantic/register liveness']},indent=2)+'\n')
results={}
for label,base,flags in [('selected-63804',selected,selected['native_flags']),('late-codebook-64444',phase,phase['native_flags']|{'MECH_GUARD_SPARSE_ROUNDS':'1'})]:
 folder=ROOT/label;folder.mkdir()
 command=[str(compiler),selected['phases']['source_to_native']['command'][1],str(folder/'frontend'),'compact']
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
 pins={name:sha(folder/'frontend'/name) for name in ['program.o','metadata.json','program.mecb']}
 expected={'program.o':base['object_sha256'],'metadata.json':base['metadata_sha256'],'program.mecb':base['bytecode_sha256']}
 assert pins==expected,(label,pins,expected)
 counts=read(folder/'frontend/construction-census.json')
 assert all(counts[arena]['resident']==0 for arena in ['blocks','instructions','values']),counts
 results[label]={'command':command,'flags':flags,'artifacts':pins,'matches_qualified_artifacts':True,'census':counts,'encoder_bytes':base['object_text_bytes']}
 (folder/'receipt.json').write_text(json.dumps(results[label],indent=2)+'\n')
(ROOT/'result.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps({label:r['census'] for label,r in results.items()},indent=2))
