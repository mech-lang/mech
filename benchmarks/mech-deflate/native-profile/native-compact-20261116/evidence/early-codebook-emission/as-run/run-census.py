"""Count total construction across physical units; diagnostic executions are not timings."""
from pathlib import Path
from hashlib import sha256
import gzip,json,os,shutil,struct,subprocess
ROOT=Path(__file__).resolve().parent
LAB=ROOT.parent
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
base=read(LAB/'early-codebook-gated-default-source/rebuild.json')
binary=LAB/'target/release/compile_profile'
compiler=ROOT/'compiler'/sha(binary)[:16]/'compile_profile'
compiler.parent.mkdir(parents=True,exist_ok=True)
shutil.copy2(binary,compiler)
patch=subprocess.check_output(['git','diff','--binary','HEAD'])
(ROOT/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
source_base=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
excluded=['MECH_COMPACT_LAYOUT_CONTROLLER_WINDOWS','MECH_COMPACT_CONTINUATION_WINDOWS','MECH_COMPACT_CHECKED_PHASE_WINDOWS']
selected=base['native_flags']
matched={k:v for k,v in selected.items() if k not in excluded}
results={}
for label,flags in [('selected-control',selected),('matched-unsplit',matched),('early-codebook',selected|{'MECH_COMPACT_EARLY_SOURCE_REGION':'4119:5031'})]:
 folder=ROOT/label
 folder.mkdir()
 command=[str(compiler),base['phases']['source_to_native']['command'][1],str(folder/'frontend'),'compact']
 env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags
 with (folder/'compile.log').open('w') as log:
  done=subprocess.run(command,env=env,stdout=log,stderr=log)
 if done.returncode:
  (folder/'failed.json').write_text(json.dumps({'command':command,'flags':flags,'exit_code':done.returncode},indent=2)+'\n')
  raise SystemExit(f'{label}: compiler failed; inspect {folder}/compile.log')
 front=folder/'frontend'
 counts=read(front/'construction-census.json')
 assert all(counts[a]['resident']==0 for a in ['values','instructions','blocks'])
 artifacts={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']}
 if label=='selected-control':
  assert artifacts=={'program.o':base['object_sha256'],'metadata.json':base['metadata_sha256'],'program.mecb':base['bytecode_sha256']},artifacts
 data=(front/'program.o').read_bytes()
 assert struct.unpack_from('<I',data)[0]==0xfeedfacf
 offset=32;text=None
 for _ in range(struct.unpack_from('<I',data,16)[0]):
  cmd,size=struct.unpack_from('<II',data,offset)
  if cmd==0x19:
   for section in range(struct.unpack_from('<I',data,offset+64)[0]):
    at=offset+72+section*80
    if data[at:at+16].rstrip(b'\0')==b'__text':text=struct.unpack_from('<Q',data,at+40)[0]
  offset+=size
 report=read(front/'metadata.json')
 r={'source_base':source_base,'source_patch_sha256':sha(ROOT/'source.patch.gz'),'compiler_sha256':sha(compiler),'command':command,'flags':flags,'artifacts':artifacts,'text_bytes':text,'census':counts,'metadata_summary':{k:report[k] for k in ['symbol','stack_bytes','code_bytes','reachable_blocks','early_source_region'] if k in report},'timing_scope':'instrumented diagnostic; not a build-time claim','runtime_qualification':'pending for changed artifacts'}
 (folder/'receipt.json').write_text(json.dumps(r,indent=2)+'\n')
 results[label]=r
 print(json.dumps({'case':label,'text_bytes':text,'census':counts,'region':report.get('early_source_region',{})},indent=2),flush=True)
(ROOT/'result.json').write_text(json.dumps(results,indent=2)+'\n')
