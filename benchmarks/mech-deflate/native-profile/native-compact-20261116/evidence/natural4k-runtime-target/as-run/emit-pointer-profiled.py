"""Emit exact disabled control and source-defined vector pointer continuations."""
from pathlib import Path
from hashlib import sha256
import json,os,shutil,subprocess,sys,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent
sys.path.insert(0,str(LAB.parent))
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
base=read(LAB/'early-codebook-gated-default-source/rebuild.json');output=ROOT/'vector-pointers'
compiler,=list((output/'compiler').glob('*/compile_profile'));original=compiler;compiler.parent.mkdir(parents=True,exist_ok=True)
if not compiler.exists():shutil.copy2(original,compiler)
assert sha(compiler)==sha(original)
parent=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
patch=subprocess.check_output(['git','diff','--binary'])
# Git abbreviation configuration can change without changing any source.
canonical=lambda data:b'\n'.join(line for line in data.splitlines() if not line.startswith(b'index '))
assert canonical(patch)==canonical((output/'source.patch').read_bytes())
subprocess.run(['git','apply','--reverse','--check',str(output/'source.patch')],check=True)
(compiler.parent/'result.json').write_text(json.dumps({'source_commit':parent,'source_patch_sha256':sha(output/'source.patch'),'compiler_sha256':sha(compiler),'compiler_build':read(output/'build-plan.json'),'scope':'Normal optimized compiler; generic opt-in vector pointer continuation. This is compiler warm rebuild provenance, not a source-through-link timing.'},indent=2)+'\n')
flags=base['native_flags'];clean={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
without=';'.join(x for x in flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';') if not x.startswith('mech_compact_numeric_3009_no_growth_composed@'))
for label,option in [('pointer-profiled',{'MECH_ORDERED_VECTOR_CARRY_POINTERS':'1'}),('pointer-lanes-profiled',{'MECH_ORDERED_VECTOR_CARRY_POINTERS':'1','MECH_ORDERED_VECTOR_CARRY_LANES':'1'})]:
 original_label=label.replace('-profiled','-default-layout');option=option|{'MECH_COMPACT_PHASE_BLOCK_LAYOUT':without+';mech_compact_numeric_3009_no_growth_composed@'+str(ROOT/(original_label+'-scorer-profile.json'))}
 folder=ROOT/label;folder.mkdir(exist_ok=True);front=folder/'frontend';command=[str(compiler),base['phases']['source_to_native']['command'][1],str(front),'compact']
 elapsed=None
 if not all((front/n).exists() for n in ['program.o','metadata.json','program.mecb']):
  start=time.perf_counter()
  with (folder/'compile.log').open('w') as log:done=subprocess.run(command,env=clean|flags|option,stdout=log,stderr=log)
  elapsed=time.perf_counter()-start
  if done.returncode:print(label,'failed',flush=True);raise SystemExit(done.returncode)
 artifacts={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']};assert artifacts['program.mecb']==base['bytecode_sha256']
 if label=='pointer-disabled-control':assert artifacts['program.o']==base['object_sha256'] and artifacts['metadata.json']==base['metadata_sha256']
 sys.path.insert(0,str(LAB.parent/'current-roadmap/scorer-runtime'))
 import emit
 layout=emit.symbols(front/'program.o')
 record={'source_commit':parent,'source_pin':parent,'source_patch_sha256':sha(output/'source.patch'),'compiler_sha256':sha(compiler),'command':command,'compile':command,'flags':flags|option,'object_sha256':artifacts['program.o'],'metadata_sha256':artifacts['metadata.json'],'bytecode_sha256':artifacts['program.mecb'],'text_bytes':layout['text_bytes'],'layout':layout,'unpaired_emission_seconds_diagnostic':elapsed,'control_receipt':str(LAB/'preserved-registers/preserved/receipt.json'),'runtime_qualification':'pending'}
 (folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(label,layout['text_bytes'],artifacts['program.o'],flush=True)
