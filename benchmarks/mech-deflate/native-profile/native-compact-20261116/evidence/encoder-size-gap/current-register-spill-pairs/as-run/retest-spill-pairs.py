"""Revisit the existing spill-pair implementation with current source/options and exact controls."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,sys,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;OUT=ROOT/'retest-register-spills';OUT.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
base=read(LAB/'early-codebook-gated-default-source/rebuild.json');compiler,=list((ROOT/'vector-pointers/compiler').glob('*/compile_profile'));producer=read(compiler.with_name('result.json'))
assert subprocess.check_output(['git','diff','--','hosts/gpu/src/native_artifact/program/codegen/ordered_reduction'],text=True)==''
plan='''# Current allocator spill pairs — before experiment\n\n1. Full objective remains1x libdeflate code/build time and more runtime wins while retaining81Roc/62C and every existing peer win. Use the selected63804B worker as the reference.\n2. The history audit finds old pairing held every full peer win but v1 rejected small slowdowns. Current all-loss profiles put72.5% of1MiB encoder samples in collector475; current counters show25.95M stack-load instructions. The live pairing flag is absent from selected flags.\n3. Retest the existing generic allocator spill-pair feature with current scoped packets, spill forwarding, direct homes and current source. Emit matched unprofiled control, paired and checker-enabled bodies from one immutable normal compiler. Existing machine-offset profiles cannot be reused blindly; rebase only after exact ordered-control-graph matching. No raw-byte implementation, no LLVM dependency, no codec source changes.\n4. Require equal normal/checker objects and metadata, source bytecode unchanged, whole encoder accounting, actual linked outputs/errors/recoveries, stack<=5280, focused then complete both-order qualification and every required peer win. Measure complete matched source-through-link time before selection.\n5. Keep all adverse data. If a lost win is due to function placement, attempt bounded source-emitted alignment recovery using actual bodies. Small slowdowns without peer losses are allowed. Do not claim old61C receipts satisfy current62C.\n'''
(OUT/'plan.md').write_text(plan)
(OUT/'plan.json').write_text(json.dumps({'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','compiler_sha256':sha(compiler),'compiler_producer':producer,'source_sha256':base['source_sha256'],'reference_worker_sha256':'5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0','reference_flags':base['native_flags'],'plan_md_sha256':sha(OUT/'plan.md')},indent=2)+'\n')
sys.path.insert(0,str(LAB.parent));sys.path.insert(0,str(LAB.parent/'current-roadmap/scorer-runtime'));import emit
clean={k:v for k,v in os.environ.items() if not k.startswith('MECH_')};flags=base['native_flags'].copy();del flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']
for role,extra in [('unprofiled-control',{}),('unprofiled-paired',{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'1'}),('unprofiled-checked',{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'})]:
 folder=OUT/role;folder.mkdir();front=folder/'frontend';command=[str(compiler),base['phases']['source_to_native']['command'][1],str(front),'compact']
 with (folder/'compile.log').open('w') as log:done=subprocess.run(command,env=clean|flags|extra,stdout=log,stderr=log)
 if done.returncode:raise RuntimeError(role+' compilation failed; preserve log')
 a={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']};assert a['program.mecb']==base['bytecode_sha256'];layout=emit.symbols(front/'program.o')
 r={'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','compiler_sha256':sha(compiler),'command':command,'compile':command,'flags':flags|extra,'object_sha256':a['program.o'],'metadata_sha256':a['metadata.json'],'bytecode_sha256':a['program.mecb'],'text_bytes':layout['text_bytes'],'layout':layout,'control_receipt':str(LAB/'preserved-registers/preserved/receipt.json'),'runtime_qualification':'not performed','compile_time_measurement':'not performed; this is emission preparation, not a matched build campaign'}
 (folder/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(role,layout['text_bytes'],a['program.o'],flush=True)
a=read(OUT/'unprofiled-paired/receipt.json');b=read(OUT/'unprofiled-checked/receipt.json')
for key in ['object_sha256','metadata_sha256','bytecode_sha256']:assert a[key]==b[key],key
(OUT/'checker-match.json').write_text(json.dumps({'matched':['object_sha256','metadata_sha256','bytecode_sha256'],'artifacts':{k:a[k] for k in ['object_sha256','metadata_sha256','bytecode_sha256']}},indent=2)+'\n')
print('Normal/checker artifacts match exactly; profiles and qualification pending.',flush=True)
