"""Untimed screen of existing generic unsigned-guard proof on the current paired recipe."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,sys
HERE=Path(__file__).resolve().parent;OUT=HERE/'hot-matcher-facts';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(p.read_bytes()).hexdigest()
base=read(HERE/'retest-register-spills/unprofiled-paired/receipt.json');compiler=Path(base['compile'][0]);assert sha(compiler)==base['compiler_sha256'];sys.path.insert(0,str(HERE.parent.parent/'current-roadmap/scorer-runtime'));import emit
for label,extra in [('unsigned-guards',{'MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS':'1'}),('unsigned-guards-checked',{'MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS':'1','MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'})]:
 folder=OUT/label;folder.mkdir();front=folder/'frontend';command=[str(compiler),base['compile'][1],str(front),'compact'];flags=base['flags']|extra
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
 artifacts={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']};assert artifacts['program.mecb']==base['bytecode_sha256'];layout=emit.symbols(front/'program.o');meta=read(front/'metadata.json')
 record={'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','compiler_sha256':sha(compiler),'command':command,'flags':flags,'artifacts':artifacts,'text_bytes':layout['text_bytes'],'layout':layout,'baseline_receipt':str(HERE/'retest-register-spills/unprofiled-paired/receipt.json'),'changed_numeric_phases':[{k:p.get(k) for k in ['header','symbol','code_bytes','unsigned_guards','vector_variant']} for p in meta['numeric_phases'] if p.get('header')==475],'qualification':'Emission and allocation-checker preflight only; no layout/runtime/stack/build-time claim.'}
 (folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(label,layout['text_bytes'],artifacts['program.o'],flush=True)
a=read(OUT/'unsigned-guards/receipt.json');b=read(OUT/'unsigned-guards-checked/receipt.json');assert a['artifacts']==b['artifacts']
(OUT/'preflight.json').write_text(json.dumps({'baseline_text_bytes':base['text_bytes'],'candidate_text_bytes':a['text_bytes'],'whole_text_delta_bytes':a['text_bytes']-base['text_bytes'],'normal_checker_artifacts_match':True,'candidate_object_changed':a['artifacts']['program.o']!=base['object_sha256'],'qualification':'Unprofiled emission screen only. The selected recipe is unchanged.'},indent=2)+'\n')
