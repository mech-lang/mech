from pathlib import Path
from hashlib import sha256
import json,os,shutil,struct,subprocess,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
base=read(LAB/'early-codebook-gated-default-source/rebuild.json')
old=LAB/'early-codebook-lowering/normal/compiler/4c737bbabe8d8693/compile_profile'
compiler=ROOT/'compiler/4c737bbabe8d8693/compile_profile';compiler.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(old,compiler)
assert sha(compiler)=='4c737bbabe8d86932f4e736546388a0f7f1afad12891736fe45c8c3fdbf3a8b4'
(compiler.parent/'result.json').write_text(json.dumps({'source_commit':'b104d014da5da9b87ef270909507e9bfe35558f8','compiler_sha256':sha(compiler),'compiler_build':base['compiler_build'],'scope':'Source files match the published b104 tree; binary copied from exact measured normal compiler4c, built with early-region-emission. No source edits for these configuration trials.'},indent=2)+'\n')
flags=base['native_flags'];clean={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
flags=flags.copy();flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(x for x in flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';') if not x.startswith('mech_compact_numeric_3009_no_growth_composed@'))
for label,option in [('control-rebased-scorer-layout',{}),('deferred-rebased-scorer-layout',{'MECH_ORDERED_PACK_WINNER_AT_EXIT':'1'})]:
 option=option.copy();original_label=label.replace('-rebased-','-default-');option['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']+';mech_compact_numeric_3009_no_growth_composed@'+str(ROOT/(original_label+'-rebased-profile.json'))
 folder=ROOT/label;folder.mkdir();front=folder/'frontend';command=[str(compiler),base['phases']['source_to_native']['command'][1],str(front),'compact']
 start=time.perf_counter()
 with (folder/'compile.log').open('w') as log:done=subprocess.run(command,env=clean|flags|option,stdout=log,stderr=log)
 elapsed=time.perf_counter()-start
 if done.returncode:
  (folder/'failure.json').write_text(json.dumps({'command':command,'flags':flags|option,'exit_code':done.returncode},indent=2)+'\n');print(label,'compile failure',flush=True);continue
 data=(front/'program.o').read_bytes();at=32;text=None
 for _ in range(struct.unpack_from('<I',data,16)[0]):
  tag,size=struct.unpack_from('<II',data,at)
  if tag==0x19:
   for i in range(struct.unpack_from('<I',data,at+64)[0]):
    s=at+72+80*i
    if data[s:s+16].rstrip(b'\0')==b'__text':text=struct.unpack_from('<Q',data,s+40)[0]
  at+=size
 artifacts={name:sha(front/name) for name in ['program.o','metadata.json','program.mecb']}
 if label=='control-rebased-scorer-layout':assert artifacts['program.o']==base['object_sha256'] and artifacts['metadata.json']==base['metadata_sha256']
 assert artifacts['program.mecb']==base['bytecode_sha256']

 record={'source_commit':'b104d014da5da9b87ef270909507e9bfe35558f8','source_pin':'b104d014da5da9b87ef270909507e9bfe35558f8','compiler_sha256':sha(compiler),'command':command,'compile':command,'flags':flags|option,'object_sha256':artifacts['program.o'],'metadata_sha256':artifacts['metadata.json'],'bytecode_sha256':artifacts['program.mecb'],'text_bytes':text,'layout':{'object_sha256':artifacts['program.o'],'text_bytes':text},'unpaired_emission_seconds_diagnostic':elapsed,'control_receipt':str(LAB/'preserved-registers/preserved/receipt.json'),'runtime_qualification':'pending'}
 (folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(label,text,artifacts['program.o'],flush=True)
